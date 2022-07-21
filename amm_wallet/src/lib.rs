use near_sdk::{AccountId, Balance, env, ext_contract, Gas, near_bindgen, PanicOnDefault, Promise};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

pub const CALLBACK_GAS: Gas = Gas(5_000_000_000_000);

#[ext_contract(ext_ft)]
pub trait FungibleToken {
    fn caller_transfer(&mut self, account_id: AccountId, balance: Balance, memo: Option<String>);
}

#[near_bindgen]
#[derive(Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[serde(crate = "near_sdk::serde")]
pub struct SubContract {
    pub a: AccountId,
    pub b: AccountId,
    pub owner: AccountId,

}

#[near_bindgen]
impl SubContract {
    #[init]
    pub fn init(a: AccountId, b: AccountId) -> Self {
        Self {
            a,
            b,
            owner: env::predecessor_account_id(),
        }
    }

    fn assert_owner(&mut self) {
        assert_eq!(env::predecessor_account_id(), self.owner, "this contract can only be called by its owner")
    }

    #[payable]
    pub fn withdraw_a(&mut self, account_id: AccountId, qty: Balance) -> Promise {
        self.assert_owner();
        ext_ft::ext(self.a.clone())
            .with_attached_deposit(1)
            .caller_transfer(
                account_id,
                qty,
                Some("withdraw from amm sub contract".to_string()),
            )
    }

    #[payable]
    pub fn withdraw_b(&mut self, account_id: AccountId, qty: Balance) -> Promise {
        self.assert_owner();
        ext_ft::ext(self.b.clone())
            .with_attached_deposit(1)
            .caller_transfer(
                account_id,
                qty,
                Some("withdraw from amm sub contract".to_string()),
            )
    }
}