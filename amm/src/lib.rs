use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider};
use near_contract_standards::fungible_token::resolver::FungibleTokenResolver;
use near_contract_standards::storage_management::StorageBalance;
use near_sdk::{AccountId, Balance, env, ext_contract, Gas, log, near_bindgen, Promise, PromiseOrValue, PromiseResult, serde_json};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};

const CODE: &[u8] = include_bytes!("../../amm_wallet/res/amm_wallet.wasm");
const N: Balance = 1_000_000_000_000_000_000_000_000;

#[ext_contract(ext_ft)]
pub trait FungibleTokenContract<T = Self>
    where
        T: FungibleTokenCore
        + FungibleTokenMetadataProvider
        + FungibleTokenResolver
{
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128>;
    fn ft_total_supply(&self) -> U128;
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
    fn ft_metadata(&self) -> FungibleTokenMetadata;
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance;
}

#[ext_contract(ext_wallet)]
pub trait AmmWalletContract {
    fn init(a: AccountId, a_meta: FungibleTokenMetadata, b: AccountId, b_meta: FungibleTokenMetadata) -> Self;
}

#[near_bindgen]
#[derive(Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Contract {
    wallet: AccountId,
    initialized: bool,
}

// Define the default, which automatically initializes the contract
impl Default for Contract {
    fn default() -> Self {
        Self {
            wallet: AccountId::new_unchecked("a".repeat(64)),
            initialized: false,
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn init(&mut self, a: AccountId, b: AccountId) -> Promise {
        assert!(!self.initialized, "contract is already initialized");

        log!("start init, a: {}, b: {}", a, b);
        let p_fetch_meta_a = ext_ft::ext(a.clone()).ft_metadata();
        let p_fetch_meta_b = ext_ft::ext(b.clone()).ft_metadata();
        let p_create_wallet = Self::ext(env::current_account_id())
            .with_static_gas(Gas(200_000_000_000_000))
            .create_wallet_with_metadata(a, b);
        let p_init_done = Self::ext(env::current_account_id())
            .with_unused_gas_weight(1)
            .init_done();
        p_fetch_meta_a.and(p_fetch_meta_b)
            .then(p_create_wallet)
            .then(p_init_done)
    }

    #[private]
    pub fn create_wallet_with_metadata(&mut self, a: AccountId, b: AccountId) -> Promise {
        assert_eq!(env::promise_results_count(), 2, "should have 2 metadata results");
        let md_a: FungibleTokenMetadata = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("Failed to get token metadata"),
            PromiseResult::Successful(result) => {
                serde_json::from_slice::<FungibleTokenMetadata>(&result)
                    .unwrap()
            }
        };
        let md_b: FungibleTokenMetadata = match env::promise_result(1) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("Failed to get token metadata"),
            PromiseResult::Successful(result) => {
                serde_json::from_slice::<FungibleTokenMetadata>(&result)
                    .unwrap()
            }
        };
        log!("a: {}, md a: {}", a, serde_json::to_string(&md_a).unwrap());
        log!("b: {}, md b: {}",b,serde_json::to_string(&md_b).unwrap());
        self.init_wallet(a, md_a, b, md_b)
    }

    fn init_wallet(&mut self, a: AccountId, a_metadata: FungibleTokenMetadata, b: AccountId, b_metadata: FungibleTokenMetadata) -> Promise {
        let wallet_account_id =
            AccountId::new_unchecked(format!("{}_{}.{}", "wallet", env::block_height() % 10000, env::current_account_id()));
        log!("wallet account id: {}", wallet_account_id);

        let p_deploy_wallet_contract = Promise::new(wallet_account_id.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .transfer(30 * N)
            .deploy_contract(CODE.to_vec());
        let p_init_wallet_contract = ext_wallet::ext(wallet_account_id.clone())
            .with_unused_gas_weight(1)
            .init(a.clone(), a_metadata, b.clone(), b_metadata);
        let p_register = self.register_account(wallet_account_id.clone(), a, b);
        let p_callback = Self::ext(env::current_account_id())
            .with_unused_gas_weight(1)
            .create_wallet_callback(wallet_account_id);

        p_deploy_wallet_contract
            .then(p_init_wallet_contract)
            .and(p_register)
            .then(p_callback)
    }

    fn register_account(&mut self, wallet_account_id: AccountId, a: AccountId, b: AccountId) -> Promise {
        let p_register_self_to_a = ext_ft::ext(a.clone())
            .with_attached_deposit(N)
            .storage_deposit(
                None,
                None,
            );
        let p_register_self_to_b = ext_ft::ext(b.clone())
            .with_attached_deposit(N)
            .storage_deposit(
                None,
                None,
            );
        let p_register_wallet_to_a = ext_ft::ext(a)
            .with_attached_deposit(N)
            .storage_deposit(
                Some(wallet_account_id.clone()),
                None,
            );
        let p_register_wallet_to_b = ext_ft::ext(b)
            .with_attached_deposit(N)
            .storage_deposit(
                Some(wallet_account_id),
                None,
            );
        p_register_self_to_a
            .and(p_register_self_to_b)
            .and(p_register_wallet_to_a)
            .and(p_register_wallet_to_b)
    }

    #[private]
    pub fn create_wallet_callback(&mut self, wallet_account_id: AccountId) {
        log!("promise result count in create_wallet_callback: {}", env::promise_results_count());
        assert_all_result_success();
        log!("create wallet {} done", wallet_account_id);
        self.wallet = wallet_account_id
    }

    #[private]
    pub fn init_done(&mut self) {
        log!("promise result count in init_done: {}", env::promise_results_count());
        assert_all_result_success();
        log!("init done");
        self.initialized = true
    }

    pub fn state(&mut self) -> Contract {
        self.clone()
    }

    pub fn update_wallet_contract(&mut self) -> Promise {
        Promise::new(self.wallet.clone())
            .add_full_access_key(env::signer_account_pk())
            .deploy_contract(CODE.to_vec())
    }
}

fn assert_all_result_success() {
    for i in 0..env::promise_results_count() {
        match env::promise_result(i) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("promise result {} failed", i),
            PromiseResult::Successful(_) => {}
        };
    }
}