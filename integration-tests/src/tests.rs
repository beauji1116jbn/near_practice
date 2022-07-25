// use near_sdk::AccountId;

use std::str::FromStr;

use near_account_id::AccountId;
use near_sdk::json_types::U128;
use near_units::parse_near;
use serde_json::json;
use workspaces::prelude::*;
use workspaces::{network::Sandbox, Account, Contract, Worker};

use crate::consts::*;
use crate::utils::*;

mod consts;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initiate environemnt
    let worker = workspaces::sandbox().await?;

    // deploy contracts
    // let ft_wasm = std::fs::read(FT_WASM_FILEPATH)?;
    let ft_contract_1 = worker.dev_deploy(FT_WASM).await?;
    let ft_contract_2 = worker.dev_deploy(FT_WASM).await?;
    // let amm_wasm = std::fs::read(AMM_WASM_FILEPATH)?;
    let amm_contract = worker.dev_deploy(AMM_WASM).await?;
    println!("deploy contracts done");

    // create accounts
    let owner = worker.root_account().unwrap();

    let amm_wallet_account_id = test_init_contracts(
        &owner,
        &worker,
        &ft_contract_1,
        &ft_contract_2,
        &amm_contract,
    )
    .await?;

    test_prepare_amm_contracts(
        &owner,
        &worker,
        &ft_contract_1,
        &ft_contract_2,
        &amm_contract,
    )
    .await?;

    test_amm_wallet_owner_deposit(
        &owner,
        &worker,
        &ft_contract_1,
        &ft_contract_2,
        &amm_contract,
        &amm_wallet_account_id,
    )
    .await?;

    test_user_swap(
        &owner,
        &worker,
        &ft_contract_1,
        &ft_contract_2,
        &amm_wallet_account_id,
    )
    .await?;

    Ok(())
}

async fn test_init_contracts(
    owner: &Account,
    worker: &Worker<Sandbox>,
    ft_1: &Contract,
    ft_2: &Contract,
    amm: &Contract,
) -> anyhow::Result<AccountId> {
    // Initialize ft contracts
    ft_init(worker, owner, ft_1).await?;
    ft_init(worker, owner, ft_2).await?;

    // check if tokens are correctly initialized
    let ft_1_total_supply: U128 = owner
        .call(worker, ft_1.id(), "ft_total_supply")
        .args_json(json!({}))?
        .transact()
        .await?
        .json()?;
    assert_eq!(ft_1_total_supply.0, FT_INIT_SUPPLY);
    let ft_1_owner_balance: U128 = ft_balance(worker, owner, ft_1.id(), owner.id()).await?;
    assert_eq!(ft_1_owner_balance.0, FT_INIT_SUPPLY);
    let ft_2_total_supply: U128 = owner
        .call(worker, ft_2.id(), "ft_total_supply")
        .args_json(json!({}))?
        .transact()
        .await?
        .json()?;
    assert_eq!(ft_2_total_supply.0, FT_INIT_SUPPLY);
    let ft_2_owner_balance: U128 = ft_balance(worker, owner, ft_2.id(), owner.id()).await?;
    assert_eq!(ft_2_owner_balance.0, FT_INIT_SUPPLY);

    // init amm contract
    amm.call(worker, "init")
        .args_json(serde_json::json!({
            "a":ft_1.id(),
            "b":ft_2.id(),
        }))?
        .gas(GAS_MAX)
        .transact()
        .await?;

    let amm_info: serde_json::Value = owner
        .call(worker, amm.id(), "state")
        .args_json(json!({
            "account_id": owner.id()
        }))?
        .transact()
        .await?
        .json()?;
    assert!(amm_info.get("initialized").unwrap().as_bool().unwrap(),);
    let amm_wallet_account_id_str = amm_info.get("wallet").unwrap().as_str().unwrap();
    assert_ne!(amm_wallet_account_id_str, "");
    let amm_wallet_account_id: AccountId = AccountId::from_str(amm_wallet_account_id_str).unwrap();

    check_amm_wallet_status(
        worker,
        owner,
        &amm_wallet_account_id,
        ft_1.id(),
        ft_2.id(),
        amm.id(),
    )
    .await?;

    println!("\tPassed ✅ test_init_contracts",);
    Ok(amm_wallet_account_id)
}

async fn test_prepare_amm_contracts(
    owner: &Account,
    worker: &Worker<Sandbox>,
    ft_1: &Contract,
    ft_2: &Contract,
    amm: &Contract,
) -> anyhow::Result<()> {
    ft_transfer(worker, ft_1.id(), owner, amm.id(), "3000", false).await?;
    let amm_ft_1_balance: U128 = ft_balance(worker, owner, ft_1.id(), amm.id()).await?;
    assert_eq!(amm_ft_1_balance, U128::from(3000));

    ft_transfer(worker, ft_2.id(), owner, amm.id(), "3000", false).await?;
    let amm_ft_2_balance: U128 = ft_balance(worker, owner, ft_2.id(), amm.id()).await?;
    assert_eq!(amm_ft_2_balance, U128::from(3000));

    println!("\tPassed ✅ test_prepare_amm_contracts",);
    Ok(())
}

async fn test_amm_wallet_owner_deposit(
    owner: &Account,
    worker: &Worker<Sandbox>,
    ft_1: &Contract,
    ft_2: &Contract,
    amm: &Contract,
    amm_wallet_account_id: &AccountId,
) -> anyhow::Result<()> {
    ft_transfer(
        worker,
        ft_1.id(),
        amm.as_account(),
        amm_wallet_account_id,
        "1000",
        true,
    )
    .await?;
    let amm_wallet_ft_1_balance: U128 =
        ft_balance(worker, owner, ft_1.id(), amm_wallet_account_id).await?;
    assert_eq!(amm_wallet_ft_1_balance, U128::from(1000));

    check_amm_wallet_balance(worker, owner, amm_wallet_account_id, 1000u128, 0u128, 0u128).await?;

    ft_transfer(
        worker,
        ft_2.id(),
        amm.as_account(),
        amm_wallet_account_id,
        "1000",
        true,
    )
    .await?;
    let amm_wallet_ft_2_balance: U128 =
        ft_balance(worker, owner, ft_1.id(), amm_wallet_account_id).await?;
    assert_eq!(amm_wallet_ft_2_balance, U128::from(1000));

    check_amm_wallet_balance(
        worker,
        owner,
        amm_wallet_account_id,
        1000u128,
        1000u128,
        1000000u128,
    )
    .await?;
    println!("\tPassed ✅ test_amm_wallet_owner_deposit",);
    Ok(())
}

async fn test_prepare_user(
    owner: &Account,
    worker: &Worker<Sandbox>,
    ft_1: &Contract,
    ft_2: &Contract,
) -> anyhow::Result<Account> {
    let res = owner
        .create_subaccount(worker, "amm_test_1")
        .initial_balance(parse_near!("300 N"))
        .transact()
        .await?;
    assert!(res.is_success());
    let test_user = res.into_result()?;

    ft_storage_deposit(worker, owner, ft_1.id(), test_user.id()).await?;
    ft_storage_deposit(worker, owner, ft_2.id(), test_user.id()).await?;

    let init_amount: u128 = 2000;
    ft_transfer(
        worker,
        ft_1.id(),
        owner,
        test_user.id(),
        init_amount.to_string().as_str(),
        false,
    )
    .await?;
    let test_user_ft_1_balance: U128 = ft_balance(worker, owner, ft_1.id(), test_user.id()).await?;
    assert_eq!(test_user_ft_1_balance, U128::from(init_amount));

    ft_transfer(
        worker,
        ft_2.id(),
        owner,
        test_user.id(),
        init_amount.to_string().as_str(),
        false,
    )
    .await?;
    let test_user_ft_2_balance: U128 = ft_balance(worker, owner, ft_2.id(), test_user.id()).await?;
    assert_eq!(test_user_ft_2_balance, U128::from(init_amount));
    println!("\tPassed ✅ test_prepare_user",);
    Ok(test_user)
}
async fn test_user_swap(
    owner: &Account,
    worker: &Worker<Sandbox>,
    ft_1: &Contract,
    ft_2: &Contract,
    amm_wallet: &AccountId,
) -> anyhow::Result<()> {
    // 1. prepare test account
    let test_user = test_prepare_user(owner, worker, ft_1, ft_2).await?;

    // swap ft 1 to ft 2
    ft_transfer(worker, ft_1.id(), &test_user, amm_wallet, "200", true).await?;

    let amm_wallet_ft_1_balance: U128 = ft_balance(worker, owner, ft_1.id(), amm_wallet).await?;
    assert_eq!(amm_wallet_ft_1_balance, U128::from(1200));
    let b_balance: u128 = 1000000 / 1200;
    let amm_wallet_ft_2_balance: U128 = ft_balance(worker, owner, ft_2.id(), amm_wallet).await?;
    assert_eq!(amm_wallet_ft_2_balance, U128::from(b_balance));

    check_amm_wallet_balance(worker, owner, amm_wallet, 1200u128, b_balance, 1000000u128).await?;

    // swap ft 1 to ft 2
    ft_transfer(worker, ft_2.id(), &test_user, amm_wallet, "300", true).await?;

    let amm_wallet_ft_2_balance: U128 = ft_balance(worker, owner, ft_2.id(), amm_wallet).await?;
    assert_eq!(amm_wallet_ft_2_balance, U128::from(b_balance + 300));
    let a_balance: u128 = 1000000 / (b_balance + 300);
    let amm_wallet_ft_1_balance: U128 = ft_balance(worker, owner, ft_1.id(), amm_wallet).await?;
    assert_eq!(amm_wallet_ft_1_balance, U128::from(a_balance));
    println!("\tPassed ✅ test_user_swap",);
    Ok(())
}
