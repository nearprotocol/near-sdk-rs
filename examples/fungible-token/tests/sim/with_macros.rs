use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk_sim::{call, to_yocto, transaction::ExecutionStatus, view, DEFAULT_GAS};

use crate::utils::{init_with_macros as init, register_user};

#[test]
fn simulate_total_supply() {
    let initial_balance = to_yocto("100");
    let (_, ft, _, _) = init(initial_balance);

    let total_supply: U128 = view!(ft.ft_total_supply()).unwrap_json();

    assert_eq!(initial_balance, total_supply.0);
}

#[test]
fn simulate_simple_transfer() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (root, ft, _, alice) = init(initial_balance);

    // Transfer from root to alice.
    // Uses default gas amount, `near_sdk_sim::DEFAULT_GAS`
    call!(
        root,
        ft.ft_transfer(alice.account_id(), transfer_amount.into(), None),
        deposit = 1
    )
    .assert_success();

    let root_balance: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    let alice_balance: U128 = view!(ft.ft_balance_of(alice.account_id())).unwrap_json();
    assert_eq!(initial_balance - transfer_amount, root_balance.0);
    assert_eq!(transfer_amount, alice_balance.0);
}

#[test]
fn simulate_close_account_empty_balance() {
    let initial_balance = to_yocto("100000");
    let (_root, ft, _, alice) = init(initial_balance);

    let outcome = call!(alice, ft.storage_unregister(None), deposit = 1);
    outcome.assert_success();
    let result: bool = outcome.unwrap_json();
    assert!(result);
}

#[test]
fn simulate_close_account_non_empty_balance() {
    let initial_balance = to_yocto("100000");
    let (root, ft, _, _alice) = init(initial_balance);

    let outcome = call!(root, ft.storage_unregister(None), deposit = 1);
    assert!(!outcome.is_ok(), "Should panic");
    assert!(format!("{:?}", outcome.status())
        .contains("Can't unregister the account with the positive balance without force"));

    let outcome = call!(root, ft.storage_unregister(Some(false)), deposit = 1);
    assert!(!outcome.is_ok(), "Should panic");
    assert!(format!("{:?}", outcome.status())
        .contains("Can't unregister the account with the positive balance without force"));
}

#[test]
fn simulate_close_account_force_non_empty_balance() {
    let initial_balance = to_yocto("100000");
    let (root, ft, _, _alice) = init(initial_balance);

    let outcome = call!(root, ft.storage_unregister(Some(true)), deposit = 1);
    assert_eq!(
        outcome.logs()[0],
        format!("Closed @{} with {}", root.account_id(), initial_balance)
    );
    outcome.assert_success();
    let result: bool = outcome.unwrap_json();
    assert!(result);

    let total_supply: U128 = view!(ft.ft_total_supply()).unwrap_json();

    assert_eq!(total_supply.0, 0);
}

#[test]
fn simulate_transfer_call_with_burned_amount() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("1000");
    let (root, ft, defi, _alice) = init(initial_balance);

    // defi contract must be registered as a FT account
    register_user(&defi.user_account);

    // root invests in defi by calling `ft_transfer_call`
    let outcome = root
        .create_transaction(ft.account_id())
        .function_call(
            "ft_transfer_call".to_string(),
            json!({
                "receiver_id": defi.account_id(),
                "amount": transfer_amount.to_string(),
                "msg": "10",
            })
            .to_string()
            .into_bytes(),
            DEFAULT_GAS / 2,
            1,
        )
        .function_call(
            "storage_unregister".to_string(),
            json!({
                "force": true
            })
            .to_string()
            .into_bytes(),
            DEFAULT_GAS / 2,
            1,
        )
        .submit();

    assert_eq!(
        outcome.logs()[1],
        format!("Closed @{} with {}", root.account_id(), initial_balance - transfer_amount)
    );

    let result: bool = outcome.unwrap_json();
    assert!(result);

    let callback_outcome = outcome.get_receipt_results().remove(1).unwrap();

    assert_eq!(callback_outcome.logs()[0], "The account of the sender was deleted");
    assert_eq!(
        callback_outcome.logs()[1],
        format!("Account @{} burned {}", root.account_id(), 10)
    );

    let used_amount: U128 = callback_outcome.unwrap_json();
    // Sender deleted the account. Even though the returned amount was 10, it was not refunded back
    // to the sender, but was taken out of the receiver's balance and was burned.
    assert_eq!(used_amount.0, transfer_amount);

    let total_supply: U128 = view!(ft.ft_total_supply()).unwrap_json();

    assert_eq!(total_supply.0, transfer_amount - 10);

    let defi_balance: U128 = view!(ft.ft_balance_of(defi.account_id())).unwrap_json();
    assert_eq!(defi_balance.0, transfer_amount - 10);
}

#[test]
fn simulate_transfer_call_with_immediate_return_and_no_refund() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("1000");
    let (root, ft, defi, _alice) = init(initial_balance);

    // defi contract must be registered as a FT account
    register_user(&defi.user_account);

    // root invests in defi by calling `ft_transfer_call`
    call!(
        root,
        ft.ft_transfer_call(
            defi.account_id(),
            transfer_amount.into(),
            None,
            "take-my-money".into()
        ),
        deposit = 1
    )
    .assert_success();

    let root_balance: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    let defi_balance: U128 = view!(ft.ft_balance_of(defi.account_id())).unwrap_json();
    assert_eq!(initial_balance - transfer_amount, root_balance.0);
    assert_eq!(transfer_amount, defi_balance.0);
}

#[test]
fn simulate_transfer_call_when_called_contract_not_registered_with_ft() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("1000");
    let (root, ft, defi, _alice) = init(initial_balance);

    // call fails because DEFI contract is not registered as FT user
    call!(
        root,
        ft.ft_transfer_call(
            defi.account_id(),
            transfer_amount.into(),
            None,
            "take-my-money".into()
        ),
        deposit = 1
    );

    // balances remain unchanged
    let root_balance: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    let defi_balance: U128 = view!(ft.ft_balance_of(defi.account_id())).unwrap_json();
    assert_eq!(initial_balance, root_balance.0);
    assert_eq!(0, defi_balance.0);
}

#[test]
fn simulate_transfer_call_with_promise_and_refund() {
    let transfer_amount = to_yocto("100");
    let refund_amount = to_yocto("50");
    let initial_balance = to_yocto("1000");
    let (root, ft, defi, _alice) = init(initial_balance);

    register_user(&defi.user_account);

    call!(
        root,
        ft.ft_transfer_call(
            defi.account_id(),
            transfer_amount.into(),
            None,
            refund_amount.to_string()
        ),
        deposit = 1
    );

    let root_balance: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    let defi_balance: U128 = view!(ft.ft_balance_of(defi.account_id())).unwrap_json();
    assert_eq!(initial_balance - transfer_amount + refund_amount, root_balance.0);
    assert_eq!(transfer_amount - refund_amount, defi_balance.0);
}

#[test]
fn simulate_transfer_call_promise_panics_for_a_full_refund() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("1000");
    let (root, ft, defi, _alice) = init(initial_balance);

    // defi contract must be registered as a FT account
    register_user(&defi.user_account);

    // root invests in defi by calling `ft_transfer_call`
    let res = call!(
        root,
        ft.ft_transfer_call(
            defi.account_id(),
            transfer_amount.into(),
            None,
            "no parsey as integer big panic oh no".to_string()
        ),
        deposit = 1
    );
    assert!(res.is_ok());

    println!("profile_data: {:#?}", res.profile_data());

    assert_eq!(res.promise_errors().len(), 1);

    if let ExecutionStatus::Failure(execution_error) =
        &res.promise_errors().remove(0).unwrap().outcome().status
    {
        assert!(execution_error.to_string().contains("ParseIntError"));
    } else {
        unreachable!();
    }

    let root_balance: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    let defi_balance: U128 = view!(ft.ft_balance_of(defi.account_id())).unwrap_json();
    assert_eq!(initial_balance, root_balance.0);
    assert_eq!(0, defi_balance.0);
}
