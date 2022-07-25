// use near_sdk::AccountId;

use near_account_id::AccountId;
use near_sdk::json_types::U128;
use near_units::parse_near;
use serde_json::json;
use workspaces::{network::Sandbox, Account, Contract, Worker};

use crate::consts::*;
use crate::GAS_MAX;

pub async fn ft_init(
    worker: &Worker<Sandbox>,
    owner: &Account,
    ft: &Contract,
) -> anyhow::Result<()> {
    assert!(ft
        .call(worker, "new_default_meta")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "total_supply": FT_INIT_SUPPLY.to_string(),
        }))?
        .gas(GAS_MAX)
        .transact()
        .await?
        .is_success());
    Ok(())
}

pub async fn ft_storage_deposit(
    worker: &Worker<Sandbox>,
    caller: &Account,
    ft: &AccountId,
    account: &AccountId,
) -> anyhow::Result<()> {
    assert!(caller
        .call(worker, ft, "storage_deposit")
        .args_json(json!({
            "account_id": account,
        }))?
        .gas(GAS_MAX)
        .deposit(parse_near!("0.00125 N"))
        .transact()
        .await?
        .is_success());
    Ok(())
}

pub async fn ft_transfer(
    worker: &Worker<Sandbox>,
    ft: &AccountId,
    sender: &Account,
    receiver: &AccountId,
    amount: &str,
    call: bool,
) -> anyhow::Result<()> {
    assert!(sender
        .call(
            worker,
            ft,
            if call {
                "ft_transfer_call"
            } else {
                "ft_transfer"
            }
        )
        .args_json(json!({
            "receiver_id": receiver,
            "amount": amount,
            "msg":"",
        }))?
        .gas(GAS_MAX)
        .deposit(1)
        .transact()
        .await?
        .is_success());
    Ok(())
}

pub async fn ft_balance(
    worker: &Worker<Sandbox>,
    caller: &Account,
    ft: &AccountId,
    account: &AccountId,
) -> anyhow::Result<U128> {
    let res = caller
        .call(worker, ft, "ft_balance_of")
        .args_json(json!({
            "account_id": account,
        }))?
        .gas(GAS_MAX)
        .transact()
        .await?;
    assert!(res.is_success());
    res.json()
}

pub async fn check_amm_wallet_balance(
    worker: &Worker<Sandbox>,
    caller: &Account,
    amm_wallet_account_id: &AccountId,
    a_balance: u128,
    b_balance: u128,
    k: u128,
) -> anyhow::Result<()> {
    let res = caller
        .call(worker, amm_wallet_account_id, "state")
        .args_json(json!({}))?
        .transact()
        .await?;
    assert!(res.is_success());
    let amm_wallet_state: serde_json::Value = res.json()?;
    assert_eq!(
        // Balance::from_str(amm_wallet_state.get("a_balance").unwrap().as_str().unwrap())?,
        amm_wallet_state.get("a_balance").unwrap().to_string(),
        a_balance.to_string(),
    );
    assert_eq!(
        amm_wallet_state.get("b_balance").unwrap().to_string(),
        b_balance.to_string(),
    );
    assert_eq!(
        amm_wallet_state.get("k").unwrap().to_string(),
        k.to_string(),
    );
    Ok(())
}

pub async fn check_amm_wallet_status(
    worker: &Worker<Sandbox>,
    caller: &Account,
    amm_wallet_account_id: &AccountId,
    a: &AccountId,
    b: &AccountId,
    owner: &AccountId,
) -> anyhow::Result<()> {
    let res = caller
        .call(worker, amm_wallet_account_id, "state")
        .args_json(json!({}))?
        .transact()
        .await?;
    assert!(res.is_success());
    let amm_wallet_state: serde_json::Value = res.json()?;
    assert_eq!(
        amm_wallet_state.get("a").unwrap().as_str().unwrap(),
        a.as_str(),
    );

    assert_eq!(
        amm_wallet_state.get("b").unwrap().as_str().unwrap(),
        b.as_str(),
    );
    assert_eq!(
        amm_wallet_state.get("owner").unwrap().as_str().unwrap(),
        owner.as_str(),
    );
    Ok(())
}
