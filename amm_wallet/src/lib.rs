use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{AccountId, Balance, env, ext_contract, Gas, log, near_bindgen, PanicOnDefault, PromiseOrValue};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};

pub const CALLBACK_GAS: Gas = Gas(5_000_000_000_000);

#[ext_contract(ext_ft)]
pub trait FungibleTokenContract {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[near_bindgen]
#[derive(Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[serde(crate = "near_sdk::serde")]
pub struct AmmWallet {
    a: AccountId,
    a_meta: FungibleTokenMetadata,
    a_balance: Balance,

    b: AccountId,
    b_meta: FungibleTokenMetadata,
    b_balance: Balance,

    k: Balance,

    owner: AccountId,
}

#[near_bindgen]
impl AmmWallet {
    #[init]
    pub fn init(a: AccountId, a_meta: FungibleTokenMetadata, b: AccountId, b_meta: FungibleTokenMetadata) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            a,
            a_meta,
            a_balance: 0u128,
            b,
            b_meta,
            b_balance: 0u128,
            k: 0u128,
            owner: env::predecessor_account_id(),
        }
    }

    pub fn state(&mut self) -> Self {
        self.clone()
    }

    #[private]
    pub fn on_transfer_a_back(&mut self, amount: U128) -> PromiseOrValue<U128> {
        self.a_balance -= Balance::from(amount);
        PromiseOrValue::Value(U128(0))
    }
    #[private]
    pub fn on_transfer_b_back(&mut self, amount: U128) -> PromiseOrValue<U128> {
        self.b_balance -= Balance::from(amount);
        PromiseOrValue::Value(U128(0))
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for AmmWallet {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        log!("ft_on_transfer called on AmmWallet, sender_id: {}, amount: {}, msg: {}", sender_id, amount.0, msg);
        if env::predecessor_account_id() == self.a {
            log!("called by a");
            self.a_balance += Balance::from(amount);
            if sender_id != self.owner {
                log!("receive not from owner");
                let b_diff = self.b_balance - self.k / (self.a_balance);
                log!("b_diff: {}", b_diff);
                if b_diff > 0 {
                    return ext_ft::ext(self.b.clone())
                        .with_attached_deposit(1)
                        .ft_transfer(
                            sender_id,
                            b_diff.into(),
                            Some("deposit b back to user".to_string()),
                        ).then(
                        Self::ext(env::current_account_id())
                            .on_transfer_b_back(b_diff.into())).into();
                }
            } else {
                log!("receive from owner, update k");
                self.k = self.a_balance * self.b_balance;
                log!("k is updated to {}", self.k)
            }
        } else if env::predecessor_account_id() == self.b {
            log!("called by b");
            self.b_balance += Balance::from(amount);
            if sender_id != self.owner {
                log!("receive not from owner");
                let a_diff = self.a_balance - self.k / (self.b_balance);
                log!("a_diff: {}", a_diff);
                if a_diff > 0 {
                    return ext_ft::ext(self.a.clone())
                        .with_attached_deposit(1)
                        .ft_transfer(
                            sender_id,
                            a_diff.into(),
                            Some("deposit a back to user".to_string()),
                        ).then(
                        Self::ext(env::current_account_id())
                            .on_transfer_a_back(a_diff.into())).into();
                }
            } else {
                log!("receive from owner, update k");
                self.k = self.a_balance * self.b_balance;
                log!("k is updated to {}", self.k)
            }
        }
        PromiseOrValue::Value(U128(0))
    }
}