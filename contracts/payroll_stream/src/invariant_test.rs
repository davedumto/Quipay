#![cfg(test)]
extern crate std;

use super::{PayrollStream, PayrollStreamClient, StreamStatus};
use payroll_vault::{PayrollVault, PayrollVaultClient};
use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

#[derive(Clone, Debug)]
enum Operation {
    Deposit(i128),
    Advance(u64),
    Create {
        rate: i128,
        duration: u64,
        cliff_offset: u64,
    },
    Withdraw(usize),
    Cancel(usize),
}

fn operation_strategy() -> impl Strategy<Value = Operation> {
    prop_oneof![
        (500i128..25_000i128).prop_map(Operation::Deposit),
        (1u64..2_500u64).prop_map(Operation::Advance),
        (10i128..300i128, 20u64..500u64, 0u64..120u64).prop_map(
            |(rate, duration, cliff_offset)| Operation::Create {
                rate,
                duration,
                cliff_offset: cliff_offset.min(duration),
            },
        ),
        (0usize..12usize).prop_map(Operation::Withdraw),
        (0usize..12usize).prop_map(Operation::Cancel),
    ]
}

fn setup(env: &Env) -> (
    PayrollStreamClient,
    PayrollVaultClient,
    Address,
    Address,
    Address,
    Address,
    token::StellarAssetClient<'_>,
    token::Client<'_>,
) {
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(env);
    let employer = Address::generate(env);
    let depositor = Address::generate(env);
    let token_admin = Address::generate(env);

    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let stellar_asset_client = token::StellarAssetClient::new(env, &token_id);
    let token_client = token::Client::new(env, &token_id);

    let vault_id = env.register_contract(None, PayrollVault);
    let stream_id = env.register_contract(None, PayrollStream);

    let vault_client = PayrollVaultClient::new(env, &vault_id);
    let stream_client = PayrollStreamClient::new(env, &stream_id);

    vault_client.initialize(&admin);
    stream_client.init(&admin);
    vault_client.set_authorized_contract(&stream_id);
    stream_client.set_vault(&vault_id);

    stellar_asset_client.mint(&depositor, &2_000_000);
    vault_client.deposit(&depositor, &token_id, &250_000);

    (
        stream_client,
        vault_client,
        employer,
        depositor,
        token_id,
        vault_id,
        stellar_asset_client,
        token_client,
    )
}

fn assert_global_invariants(
    stream_client: &PayrollStreamClient,
    vault_client: &PayrollVaultClient,
    token_client: &token::Client<'_>,
    token_id: &Address,
    stream_ids: &[u64],
) {
    let vault_balance = vault_client.get_treasury_balance(token_id);
    let vault_liability = vault_client.get_total_liability(token_id);

    let mut aggregate_active_liability = 0i128;
    for stream_id in stream_ids {
        let Some(stream) = stream_client.get_stream(stream_id) else {
            continue;
        };

        assert!(
            stream.withdrawn_amount <= stream.total_amount,
            "worker withdrawal total exceeded stream amount for stream {}",
            stream_id,
        );

        if stream.status == StreamStatus::Active {
            aggregate_active_liability += stream.total_amount - stream.withdrawn_amount;
        }
    }

    assert!(
        aggregate_active_liability <= vault_liability,
        "sum of active stream liabilities ({}) exceeded vault total liability ({})",
        aggregate_active_liability,
        vault_liability,
    );
    assert!(
        vault_balance >= vault_liability,
        "vault balance ({}) dropped below vault liability ({})",
        vault_balance,
        vault_liability,
    );
    assert_eq!(
        token_client.balance(&vault_client.get_contract_address()),
        vault_balance,
        "vault token balance diverged from treasury balance accounting",
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn prop_stream_vault_invariants_hold(
        operations in prop::collection::vec(operation_strategy(), 5..40)
    ) {
        let env = Env::default();
        let (
            stream_client,
            vault_client,
            employer,
            depositor,
            token_id,
            _vault_id,
            stellar_asset_client,
            token_client,
        ) = setup(&env);

        let mut current_time = 1_000u64;
        env.ledger().set_timestamp(current_time);

        let mut stream_ids: std::vec::Vec<u64> = std::vec::Vec::new();

        for operation in operations {
            match operation {
                Operation::Deposit(amount) => {
                    stellar_asset_client.mint(&depositor, &amount);
                    let _ = vault_client.try_deposit(&depositor, &token_id, &amount);
                }
                Operation::Advance(delta) => {
                    current_time = current_time.saturating_add(delta);
                    env.ledger().set_timestamp(current_time);
                }
                Operation::Create {
                    rate,
                    duration,
                    cliff_offset,
                } => {
                    let worker = Address::generate(&env);
                    let start_ts = current_time;
                    let end_ts = current_time.saturating_add(duration);
                    let cliff_ts = start_ts.saturating_add(cliff_offset.min(duration));

                    if let Ok(Ok(stream_id)) = stream_client.try_create_stream(
                        &employer, &worker, &token_id, &rate, &cliff_ts, &start_ts, &end_ts,
                    ) {
                        stream_ids.push(stream_id);
                    }
                }
                Operation::Withdraw(index) => {
                    if stream_ids.is_empty() {
                        continue;
                    }

                    let stream_id = stream_ids[index % stream_ids.len()];
                    if let Some(stream) = stream_client.get_stream(&stream_id) {
                        let _ = stream_client.try_withdraw(&stream_id, &stream.worker);
                    }
                }
                Operation::Cancel(index) => {
                    if stream_ids.is_empty() {
                        continue;
                    }

                    let stream_id = stream_ids[index % stream_ids.len()];
                    let Some(stream_before_cancel) = stream_client.get_stream(&stream_id) else {
                        continue;
                    };

                    if stream_before_cancel.status != StreamStatus::Active {
                        continue;
                    }

                    let vault_balance_before = vault_client.get_treasury_balance(&token_id);
                    let vault_liability_before = vault_client.get_total_liability(&token_id);
                    let worker_balance_before =
                        token_client.balance(&stream_before_cancel.worker);
                    let vested = stream_client.get_withdrawable(&stream_id).unwrap_or(0);
                    let liability_for_stream =
                        stream_before_cancel.total_amount - stream_before_cancel.withdrawn_amount;

                    let result = stream_client.try_cancel_stream(&stream_id, &employer, &None);
                    if let Ok(Ok(())) = result {
                        let stream_after_cancel = stream_client
                            .get_stream(&stream_id)
                            .expect("stream should still exist after cancellation");
                        let worker_balance_after =
                            token_client.balance(&stream_after_cancel.worker);
                        let vault_balance_after = vault_client.get_treasury_balance(&token_id);
                        let vault_liability_after = vault_client.get_total_liability(&token_id);

                        assert_eq!(stream_after_cancel.status, StreamStatus::Canceled);
                        assert_eq!(
                            vault_liability_after,
                            vault_liability_before - liability_for_stream,
                            "cancelled stream did not fully remove its remaining liability from the vault",
                        );
                        assert_eq!(
                            worker_balance_after,
                            worker_balance_before + vested,
                            "worker did not receive the vested portion on cancellation",
                        );
                        assert_eq!(
                            vault_balance_after,
                            vault_balance_before - vested,
                            "cancelled stream did not leave the unvested balance in the vault",
                        );
                    }
                }
            }

            assert_global_invariants(
                &stream_client,
                &vault_client,
                &token_client,
                &token_id,
                &stream_ids,
            );
        }
    }
}
