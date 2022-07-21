use std::ops::{Add, Div, Mul};

use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider};

use near_contract_standards::fungible_token::resolver::FungibleTokenResolver;
use near_sdk::{AccountId, assert_one_yocto, assert_self, Balance, env, ext_contract, Gas, is_promise_success, log, near_bindgen, Promise, PromiseOrValue, PromiseResult};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

pub const CALLBACK_GAS: Gas = Gas(5_000_000_000_000);

const INITIAL_BALANCE: Balance = 100_000_000_000_000_000_000_000_000; // 2.5e23yN, 0.25N

const CODE: &[u8] = include_bytes!("../../amm_wallet/res/amm_wallet.wasm");

#[ext_contract(ext_ft)]
pub trait FungibleToken<T = Self>
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
}

#[ext_contract(ext_sub)]
pub trait SubContract {
    fn init(a: AccountId, b: AccountId) -> Self;

    fn withdraw_a(&mut self, account_id: AccountId, qty: Balance) -> Promise;

    fn withdraw_b(&mut self, account_id: AccountId, qty: Balance) -> Promise;
}

#[near_bindgen]
#[derive(Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Contract {
    pub a: Token,
    pub b: Token,
    pub wallet: AccountId,
    initialized: bool,
}

#[near_bindgen]
#[derive(Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    addr: AccountId,
    meta: Option<FungibleTokenMetadata>,
    qty: Balance,
}

// Define the default, which automatically initializes the contract
impl Default for Contract {
    fn default() -> Self {
        Self {
            a: Token {
                addr: env::current_account_id(),
                meta: None,
                qty: Default::default(),
            },
            b: Token {
                addr: env::current_account_id(),
                meta: None,
                qty: Default::default(),
            },
            wallet: env::current_account_id(),
            initialized: false,
        }
    }
}

#[near_bindgen]
impl Contract {
    fn assert_initialized(&mut self) {
        assert!(self.initialized, "contract is not initialized yet")
    }
    fn assert_deposited(&mut self){
        assert!(self.a.qty * self.b.qty > 0, "must deposit for both tokens before swap")
    }
    pub fn init(&mut self, a: String, b: String) -> Promise {
        log!("start init, a: {}, b: {}", a, b);
        let a_account_id = AccountId::try_from(a).unwrap();
        let b_account_id = AccountId::try_from(b).unwrap();

        self.a.addr = a_account_id.clone();
        self.b.addr = b_account_id.clone();
        let p_set_a_meta = self.solve_token_metadata(a_account_id.clone());
        let p_set_b_meta = self.solve_token_metadata(b_account_id.clone());
        let p_create_wallet = self.create_wallet(a_account_id, b_account_id);
        p_set_a_meta.and(p_set_b_meta).and(p_create_wallet).then(
            Self::ext(env::current_account_id())
                .with_unused_gas_weight(1)
                .init_done(),
        )
    }

    #[private]
    pub fn solve_token_metadata(&mut self, addr: AccountId) -> Promise {
        ext_ft::ext(addr.clone()).ft_metadata().then(
            Self::ext(env::current_account_id())
                // .with_static_gas(CALLBACK_GAS)
                .with_unused_gas_weight(1)
                .solve_token_metadata_callback(addr),
        )
    }
    #[private]
    pub fn solve_token_metadata_callback(&mut self, addr: AccountId) {
        // handle the result from the cross contract call this method is a callback for
        let meta: FungibleTokenMetadata = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("Failed to get token metadata"),
            PromiseResult::Successful(result) => {
                near_sdk::serde_json::from_slice::<FungibleTokenMetadata>(&result)
                    .unwrap()
            }
        };
        log!(
            "got metadata of {}: {}",
            addr,
            near_sdk::serde_json::to_string(&meta).unwrap()
        );
        if addr == self.a.addr {
            self.a.meta = Option::from(meta)
        } else if addr == self.b.addr {
            self.b.meta = Option::from(meta)
        } else {
            panic!("Unexpected address {}", addr)
        }
    }
    #[private]
    pub fn create_wallet(&mut self, a: AccountId, b: AccountId) -> Promise {
        let wallet_account_id =
            AccountId::new_unchecked(format!("{}_{}.{}", "wallet", env::block_height() % 10000, env::current_account_id()));

        let p_create_account = Promise::new(wallet_account_id.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .transfer(INITIAL_BALANCE)
            .deploy_contract(CODE.to_vec());
        let p_deploy_code = ext_sub::ext(wallet_account_id.clone())
            .init(a, b);
        p_create_account.then(p_deploy_code).then(
            Self::ext(env::current_account_id())
                .with_unused_gas_weight(1)
                .create_wallet_callback(wallet_account_id),
        )
    }
    #[private]
    pub fn create_wallet_callback(&mut self, wallet_account_id: AccountId) {
        log!("create wallet {} done", wallet_account_id);
        self.wallet = wallet_account_id
    }
    #[private]
    pub fn init_done(&mut self) {
        log!("init done");
        self.initialized = true
    }

    pub fn stat(&mut self) -> Contract {
        self.clone()
    }

    #[payable]
    pub fn deposit_a(&mut self, qty: U128) -> Promise {
        self.assert_initialized();
        // 1. assert self
        assert_self();
        assert_one_yocto();
        // 2. validate input
        assert!(qty > U128(0), "At least one quantity of a and b must be positive");
        ext_ft::ext(self.a.addr.clone())
            .with_attached_deposit(1)
            .ft_transfer(
                self.wallet.clone(),
                qty,
                Some("deposit from amm contract".to_string()),
            )
            .then(Self::ext(env::current_account_id()).deposit_a_callback(qty))
    }

    #[private]
    pub fn deposit_a_callback(&mut self, qty: U128) {
        assert!(is_promise_success(), "failed to transfer");
        self.a.qty += Balance::from(qty)
    }

    #[payable]
    pub fn deposit_b(&mut self, qty: U128) -> Promise {
        self.assert_initialized();
        // 1. assert self
        assert_self();
        assert_one_yocto();
        // 2. validate input
        assert!(qty > U128(0), "At least one quantity of a and b must be positive");
        ext_ft::ext(self.b.addr.clone())
            .with_attached_deposit(1)
            .ft_transfer(
                self.wallet.clone(),
                qty,
                Some("deposit from amm contract".to_string()),
            )
            .then(
                Self::ext(env::current_account_id())
                    .deposit_b_callback(qty))
    }

    #[private]
    pub fn deposit_b_callback(&mut self, qty: U128) {
        assert!(is_promise_success(), "failed to transfer");
        self.b.qty += Balance::from(qty)
    }

    #[payable]
    pub fn swap_a(&mut self, qty: U128) -> Promise {
        self.assert_initialized();
        self.assert_deposited();
        assert!(qty > U128(0), "input quantity must be positive");

        let b_diff = get_des_diff_from_src_qty(self.a.qty, qty.into(), self.b.qty);
        log!("current a qty: {}, b qty: {}, get b diff in swap a: {}",self.a.qty, self.b.qty, b_diff);

        // try to send qty a from user to wallet
        let p_send_a = ext_ft::ext(self.a.addr.clone())
            .with_attached_deposit(1)
            .with_unused_gas_weight(1)
            .ft_resolve_transfer(
                env::predecessor_account_id(),
                self.wallet.clone(),
                qty,
            );
        // try to send b_diff b from wallet to user
        let p_receive_b = ext_sub::ext(self.wallet.clone())
            .with_attached_deposit(1)
            .with_unused_gas_weight(1)
            .withdraw_b(
                env::signer_account_id(),
                b_diff,
            );
        // try to update balance
        p_send_a
            .then(p_receive_b)
            .then(
                Self::ext(env::current_account_id())
                    .with_unused_gas_weight(1)
                    .swap_a_callback(qty, b_diff.into()))
    }

    #[private]
    pub fn swap_a_callback(&mut self, qty: U128, diff: U128) {
        log!("swap callback promise result count: {}", env::promise_results_count() );
        self.a.qty += Balance::from(qty);
        self.b.qty -= Balance::from(diff);
    }

    #[payable]
    pub fn swap_b(&mut self, qty: U128) -> Promise {
        self.assert_initialized();
        self.assert_deposited();
        assert!(qty > U128(0), "input quantity must be positive");

        let a_diff = get_des_diff_from_src_qty(self.b.qty, qty.into(), self.a.qty);
        log!("current a qty: {}, b qty: {}, get a diff in swap b: {}",self.a.qty, self.b.qty, a_diff);
        // try to send qty a from user to wallet
        let p_send_b = ext_ft::ext(self.b.addr.clone())
            .with_attached_deposit(1)
            .with_unused_gas_weight(1)
            .ft_resolve_transfer(
                env::predecessor_account_id(),
                self.wallet.clone(),
                qty,
            );
        // try to send b_diff b from wallet to user
        let p_receive_a = ext_sub::ext(self.wallet.clone())
            .with_attached_deposit(1)
            .with_unused_gas_weight(1)
            .withdraw_a(
                env::signer_account_id(),
                a_diff,
            );
        p_send_b
            .then(p_receive_a)
            .then(
                Self::ext(env::current_account_id())
                    .with_unused_gas_weight(1)
                    .swap_b_callback(qty, a_diff.into()))
    }

    #[private]
    pub fn swap_b_callback(&mut self, qty: U128, diff: U128) {
        log!("swap callback promise result count: {}", env::promise_results_count() );
        self.b.qty += Balance::from(qty);
        self.a.qty -= Balance::from(diff);
    }
}

// des_qty = des_balance - src_balance * des_balance / (src_balance + src_qty)
//         = des_balance * src_qty / (src_balance + src_qty)
fn get_des_diff_from_src_qty(src_balance: Balance, src_qty: Balance, des_balance: Balance) -> Balance {
    let src_diff_dec = Decimal::from(src_qty);
    let src_qty_dec = Decimal::from(src_balance);
    let des_qty_dec = Decimal::from(des_balance);
    des_qty_dec.mul(src_diff_dec)
        .div(
            src_qty_dec.add(src_diff_dec)
        ).to_u128().unwrap()
}