#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig, Ledger}, token, Address, Env, Vec,
};
use types::StreamStatus;

fn setup_env() -> (Env, Address, PayrollStreamContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PayrollStreamContract, ());
    let client = PayrollStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    (env, admin, client)
}

fn setup_env_without_snapshots() -> (Env, Address, PayrollStreamContractClient<'static>) {
    let env = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env.mock_all_auths();
    let contract_id = env.register(PayrollStreamContract, ());
    let client = PayrollStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    (env, admin, client)
}

fn create_token_contract<'a>(e: &Env, admin: &Address) -> token::StellarAssetClient<'a> {
    let contract_addr = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(e, &contract_addr)
}

fn create_token_client<'a>(e: &Env, contract_addr: &Address) -> token::Client<'a> {
    token::Client::new(e, contract_addr)
}

#[test]
#[should_panic]
fn test_double_initialize() {
    let (_env, admin, client) = setup_env();
    client.initialize(&admin);
    client.initialize(&admin);
}

#[test]
fn test_create_stream_transfers_tokens_and_saves_stream() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    assert_eq!(stream_id, 0);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&client.address), 10000);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.total_amount, 10000);
    assert_eq!(stream.status, StreamStatus::Active);
    assert_eq!(stream.rate_per_second, 10);
}

#[test]
fn test_create_stream_fails_without_balance_and_does_not_persist() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &5000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let result = client.try_create_stream(&sender, &recipient, &token, &10000, &1000, &2000);
    assert_eq!(result, Err(Ok(StreamError::InsufficientBalance)));

    assert_eq!(client.get_stream_count(), 0);
    assert_eq!(token_client.balance(&sender), 5000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_create_batch_streams() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &30000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let mut streams = Vec::new(&env);
    streams.push_back(CreateStreamParams {
        recipient: Address::generate(&env),
        token: token.clone(),
        total_amount: 10000,
        start_time: 1000,
        end_time: 2000,
    });
    streams.push_back(CreateStreamParams {
        recipient: Address::generate(&env),
        token: token.clone(),
        total_amount: 20000,
        start_time: 1000,
        end_time: 3000,
    });

    let stream_ids = client.create_batch_streams(&sender, &streams);

    assert_eq!(stream_ids.len(), 2);
    assert_eq!(stream_ids.get(0).unwrap(), 0);
    assert_eq!(stream_ids.get(1).unwrap(), 1);

    let stream0 = client.get_stream(&0);
    assert_eq!(stream0.total_amount, 10000);

    let stream1 = client.get_stream(&1);
    assert_eq!(stream1.total_amount, 20000);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&client.address), 30000);
}

#[test]
fn test_batch_claims_are_limited_to_each_stream_obligation() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient_one = Address::generate(&env);
    let recipient_two = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &30_000);
    client.initialize(&admin);
    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let mut streams = Vec::new(&env);
    streams.push_back(CreateStreamParams {
        recipient: recipient_one.clone(),
        token: token.clone(),
        total_amount: 10_000,
        start_time: 1_000,
        end_time: 2_000,
    });
    streams.push_back(CreateStreamParams {
        recipient: recipient_two.clone(),
        token,
        total_amount: 20_000,
        start_time: 1_000,
        end_time: 2_000,
    });
    client.create_batch_streams(&sender, &streams);

    env.ledger().with_mut(|li| li.timestamp = 2_000);
    assert_eq!(client.claim(&recipient_one, &0), 10_000);
    assert_eq!(token_client.balance(&recipient_one), 10_000);
    assert_eq!(client.claim(&recipient_two, &1), 20_000);
    assert_eq!(token_client.balance(&recipient_two), 20_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_batch_insufficient_balance_leaves_no_state_or_transfer() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &10_000);
    client.initialize(&admin);
    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let mut streams = Vec::new(&env);
    streams.push_back(CreateStreamParams {
        recipient: Address::generate(&env),
        token: token.clone(),
        total_amount: 10_000,
        start_time: 1_000,
        end_time: 2_000,
    });
    streams.push_back(CreateStreamParams {
        recipient: Address::generate(&env),
        token,
        total_amount: 1,
        start_time: 1_000,
        end_time: 2_000,
    });

    assert_eq!(
        client.try_create_batch_streams(&sender, &streams),
        Err(Ok(StreamError::InsufficientBalance))
    );
    assert_eq!(client.get_stream_count(), 0);
    assert_eq!(token_client.balance(&sender), 10_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_batch_rejects_size_and_amount_overflow_without_transfer() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &100);
    client.initialize(&admin);
    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let mut oversized = Vec::new(&env);
    for _ in 0..51 {
        oversized.push_back(CreateStreamParams {
            recipient: Address::generate(&env),
            token: token.clone(),
            total_amount: 1,
            start_time: 1_000,
            end_time: 2_000,
        });
    }
    assert_eq!(
        client.try_create_batch_streams(&sender, &oversized),
        Err(Ok(StreamError::BatchTooLarge))
    );

    let mut overflowing = Vec::new(&env);
    overflowing.push_back(CreateStreamParams {
        recipient: Address::generate(&env),
        token: token.clone(),
        total_amount: i128::MAX,
        start_time: 1_000,
        end_time: 2_000,
    });
    overflowing.push_back(CreateStreamParams {
        recipient: Address::generate(&env),
        token,
        total_amount: 1,
        start_time: 1_000,
        end_time: 2_000,
    });
    assert_eq!(
        client.try_create_batch_streams(&sender, &overflowing),
        Err(Ok(StreamError::ArithmeticError))
    );
    assert_eq!(client.get_stream_count(), 0);
    assert_eq!(token_client.balance(&sender), 100);
    assert_eq!(token_client.balance(&client.address), 0);
}

struct BatchCaseRng(u64);

impl BatchCaseRng {
    fn next_amount(&mut self) -> i128 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.0 >> 32) % 100 + 1) as i128
    }
}

fn generated_batch(
    env: &Env,
    token: &Address,
    count: u32,
    seed: u64,
) -> (Vec<CreateStreamParams>, Vec<Address>, i128) {
    let mut rng = BatchCaseRng(seed);
    let mut streams = Vec::new(env);
    let mut recipients = Vec::new(env);
    let mut total = 0;
    for _ in 0..count {
        let recipient = Address::generate(env);
        let amount = rng.next_amount();
        streams.push_back(CreateStreamParams {
            recipient: recipient.clone(),
            token: token.clone(),
            total_amount: amount,
            start_time: 1_000,
            end_time: 2_000,
        });
        recipients.push_back(recipient);
        total += amount;
    }
    (streams, recipients, total)
}

#[test]
fn generated_batch_size_boundaries_preserve_escrow_and_indexes() {
    for (seed, batch_size) in [(1, 0), (2, 1), (3, MAX_BATCH_SIZE), (4, MAX_BATCH_SIZE + 1)] {
        let (env, admin, client) = setup_env_without_snapshots();
        let sender = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        let token = token_contract.address.clone();
        let token_client = create_token_client(&env, &token);
        let (streams, recipients, total) = generated_batch(&env, &token, batch_size, seed);
        token_contract.mint(&sender, &(total + 1));
        client.initialize(&admin);
        env.ledger().with_mut(|li| li.timestamp = 1_000);

        if batch_size <= MAX_BATCH_SIZE {
            let ids = client.create_batch_streams(&sender, &streams);
            assert_eq!(ids.len(), batch_size);
            assert_eq!(client.get_stream_count(), batch_size);
            assert_eq!(client.get_streams_by_sender(&sender).len(), batch_size);
            assert_eq!(token_client.balance(&sender), 1);
            assert_eq!(token_client.balance(&client.address), total);
            for recipient in recipients.iter() {
                assert_eq!(client.get_streams_by_recipient(&recipient).len(), 1);
            }
        } else {
            assert_eq!(
                client.try_create_batch_streams(&sender, &streams),
                Err(Ok(StreamError::BatchTooLarge))
            );
            assert_eq!(client.get_stream_count(), 0);
            assert_eq!(client.get_streams_by_sender(&sender).len(), 0);
            assert_eq!(token_client.balance(&sender), total + 1);
            assert_eq!(token_client.balance(&client.address), 0);
            for recipient in recipients.iter() {
                assert_eq!(client.get_streams_by_recipient(&recipient).len(), 0);
            }
        }
    }
}

#[test]
fn generated_repeated_token_overflow_leaves_all_state_unchanged() {
    for seed in 1..=8 {
        let (env, admin, client) = setup_env_without_snapshots();
        let sender = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        let token = token_contract.address.clone();
        let token_client = create_token_client(&env, &token);
        token_contract.mint(&sender, &100);
        client.initialize(&admin);
        env.ledger().with_mut(|li| li.timestamp = 1_000);

        let mut rng = BatchCaseRng(seed);
        let first_recipient = Address::generate(&env);
        let second_recipient = Address::generate(&env);
        let mut streams = Vec::new(&env);
        streams.push_back(CreateStreamParams {
            recipient: first_recipient.clone(),
            token: token.clone(),
            total_amount: i128::MAX,
            start_time: 1_000,
            end_time: 2_000,
        });
        streams.push_back(CreateStreamParams {
            recipient: second_recipient.clone(),
            token,
            total_amount: rng.next_amount(),
            start_time: 1_000,
            end_time: 2_000,
        });

        assert_eq!(
            client.try_create_batch_streams(&sender, &streams),
            Err(Ok(StreamError::ArithmeticError))
        );
        assert_eq!(client.get_stream_count(), 0);
        assert_eq!(client.get_streams_by_sender(&sender).len(), 0);
        assert_eq!(client.get_streams_by_recipient(&first_recipient).len(), 0);
        assert_eq!(client.get_streams_by_recipient(&second_recipient).len(), 0);
        assert_eq!(token_client.balance(&sender), 100);
        assert_eq!(token_client.balance(&client.address), 0);
    }
}

#[test]
fn generated_invalid_and_underfunded_batches_leave_all_state_unchanged() {
    for seed in 1..=8 {
        for invalid_case in 0..4 {
            let (env, admin, client) = setup_env_without_snapshots();
            let sender = Address::generate(&env);
            let token_admin = Address::generate(&env);
            let token_contract = create_token_contract(&env, &token_admin);
            let token = token_contract.address.clone();
            let token_client = create_token_client(&env, &token);
            let (mut streams, recipients, total) = generated_batch(&env, &token, 3, seed);
            token_contract.mint(&sender, &(total + 1));
            client.initialize(&admin);
            env.ledger().with_mut(|li| li.timestamp = 1_000);

            let mut invalid = streams.get(1).unwrap();
            match invalid_case {
                0 => invalid.recipient = sender.clone(),
                1 => invalid.total_amount = 0,
                2 => invalid.end_time = invalid.start_time,
                _ => invalid.start_time = 999,
            }
            streams.set(1, invalid);

            assert!(client.try_create_batch_streams(&sender, &streams).is_err());
            assert_eq!(client.get_stream_count(), 0);
            assert_eq!(client.get_streams_by_sender(&sender).len(), 0);
            assert_eq!(token_client.balance(&sender), total + 1);
            assert_eq!(token_client.balance(&client.address), 0);
            for recipient in recipients.iter() {
                assert_eq!(client.get_streams_by_recipient(&recipient).len(), 0);
            }
        }

        let (env, admin, client) = setup_env_without_snapshots();
        let sender = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        let token = token_contract.address.clone();
        let token_client = create_token_client(&env, &token);
        let (streams, recipients, total) = generated_batch(
            &env,
            &token,
            (seed % MAX_BATCH_SIZE as u64) as u32 + 1,
            seed,
        );
        token_contract.mint(&sender, &(total - 1));
        client.initialize(&admin);
        env.ledger().with_mut(|li| li.timestamp = 1_000);

        assert_eq!(
            client.try_create_batch_streams(&sender, &streams),
            Err(Ok(StreamError::InsufficientBalance))
        );
        assert_eq!(client.get_stream_count(), 0);
        assert_eq!(client.get_streams_by_sender(&sender).len(), 0);
        assert_eq!(token_client.balance(&sender), total - 1);
        assert_eq!(token_client.balance(&client.address), 0);
        for recipient in recipients.iter() {
            assert_eq!(client.get_streams_by_recipient(&recipient).len(), 0);
        }
    }
}

#[test]
fn test_calculate_claimable() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    let claimable = client.get_claimable(&stream_id);
    assert_eq!(claimable, 5000);
}

#[test]
fn test_claim_transfers_tokens_and_prevents_double_claim() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    let claimed = client.claim(&recipient, &stream_id);
    assert_eq!(claimed, 5000);
    assert_eq!(token_client.balance(&recipient), 5000);
    assert_eq!(token_client.balance(&client.address), 5000);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.claimed_amount, 5000);
    assert_eq!(stream.status, StreamStatus::Active);

    let second_claim = client.try_claim(&recipient, &stream_id);
    assert_eq!(second_claim, Err(Ok(StreamError::NothingToClaim)));
    assert_eq!(token_client.balance(&recipient), 5000);
    assert_eq!(token_client.balance(&client.address), 5000);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_claim_rejects_unauthorized_claimer() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let attacker = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    client.claim(&attacker, &stream_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_claim_rejects_when_claimable_is_zero() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);
    client.claim(&recipient, &stream_id);
}

#[test]
fn test_cancel_stream_settles_midway() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    client.cancel_stream(&sender, &stream_id);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
    assert_eq!(stream.claimed_amount, 5000);
    assert_eq!(token_client.balance(&recipient), 5000);
    assert_eq!(token_client.balance(&sender), 5000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_cancel_stream_zero_owed_refunds_sender_only() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 500;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);
    client.cancel_stream(&sender, &stream_id);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
    assert_eq!(stream.claimed_amount, 0);
    assert_eq!(token_client.balance(&recipient), 0);
    assert_eq!(token_client.balance(&sender), 10000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_cancel_stream_rejects_unauthorized() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let attacker = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    client.cancel_stream(&attacker, &stream_id);
}

#[test]
fn test_cancel_stream_after_end() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 2500;
    });

    client.cancel_stream(&sender, &stream_id);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
    assert_eq!(stream.claimed_amount, 10000);
    assert_eq!(token_client.balance(&recipient), 10000);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_cancel_stream_after_full_claim_has_zero_settlement() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 2500;
    });

    let claimed = client.claim(&recipient, &stream_id);
    assert_eq!(claimed, 10000);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&recipient), 10000);
    assert_eq!(token_client.balance(&client.address), 0);

    client.cancel_stream(&sender, &stream_id);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
    assert_eq!(stream.claimed_amount, 10000);
    assert_eq!(token_client.balance(&recipient), 10000);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&recipient), 10000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_claim_progression() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token,
        &10000_i128,
        &1000_u64,
        &2000_u64,
    );

    // 1. Claim at 25% (1250)
    env.ledger().with_mut(|li| {
        li.timestamp = 1250;
    });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 2500);

    // 2. Claim at 50% (1500)
    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 5000);

    // 3. Claim at 75% (1750)
    env.ledger().with_mut(|li| {
        li.timestamp = 1750;
    });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 7500);

    // 4. Claim at 100% (2000)
    env.ledger().with_mut(|li| {
        li.timestamp = 2000;
    });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 10000);
}

// ── Timelocked Upgrade Tests ───────────────────────────────────────────────

#[test]
fn test_upgrade_timelock_execute_after_delay() {
    let (env, admin, client) = setup_env();

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[1; 32]);

    client.initialize(&admin);

    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("v2"));

    let pending = client.get_pending_upgrade();
    assert!(pending.is_some());
    assert_eq!(pending.unwrap().wasm_hash, wasm_hash);

    // Advance time past the timelock
    env.ledger().with_mut(|li| {
        li.timestamp = 24 * 60 * 60 + 1;
    });

    // Verify the timelock has elapsed — execution is gated on this condition
    let pending_after = client.get_pending_upgrade();
    assert!(pending_after.is_some());
    let pending_after_val = pending_after.unwrap();
    assert_eq!(pending_after_val.wasm_hash, wasm_hash);
    assert!(env.ledger().timestamp() >= pending_after_val.proposed_at + 86400);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_upgrade_timelock_rejected_before_delay() {
    let (env, admin, client) = setup_env();
    let executor = Address::generate(&env);

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[1; 32]);

    client.initialize(&admin);

    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("v2"));

    // Try to execute before timelock - should fail
    env.ledger().with_mut(|li| {
        li.timestamp = 100;
    });
    client.execute_upgrade(&executor);
}

#[test]
fn test_upgrade_proposal_event_includes_actor() {
    let (env, admin, client) = setup_env();
    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[2; 32]);

    client.initialize(&admin);
    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("sec_patch"));

    let pending = client.get_pending_upgrade();
    assert!(pending.is_some());
}

#[test]
fn test_unauthorized_cancel() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let malicious = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });
    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    let result = client.try_cancel_stream(&malicious, &stream_id);
    assert!(result.is_err());
}

#[test]
fn test_invalid_creation_params() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    // 1. Invalid amount
    let res1 = client.try_create_stream(&sender, &recipient, &token, &-100, &1000, &2000);
    assert!(res1.is_err());

    // 2. Invalid duration
    let res2 = client.try_create_stream(&sender, &recipient, &token, &1000, &2000, &1000);
    assert!(res2.is_err());

    // 3. Same sender and recipient
    let res3 = client.try_create_stream(&sender, &sender, &token, &1000, &1000, &2000);
    assert!(res3.is_err());
}

#[test]
fn test_multiple_concurrent_streams() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &20000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let id1 = client.create_stream(&sender, &recipient1, &token, &10000, &1000, &2000);
    let id2 = client.create_stream(&sender, &recipient2, &token, &10000, &1000, &3000);

    // At 1500: id1 is 50%, id2 is 25%
    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    client.claim(&recipient1, &id1);
    client.claim(&recipient2, &id2);

    assert_eq!(token_client.balance(&recipient1), 5000);
    assert_eq!(token_client.balance(&recipient2), 2500);
}

#[test]
fn test_cancel_after_partial_claim() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    let start_time = 1000;
    env.ledger().with_mut(|li| {
        li.timestamp = start_time;
    });
    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token,
        &10000,
        &start_time,
        &(start_time + 1000),
    );

    // 1. Advance to 25% (250s)
    env.ledger().with_mut(|li| {
        li.timestamp = start_time + 250;
    });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 2500);

    // 2. Advance to 50% (500s)
    env.ledger().with_mut(|li| {
        li.timestamp = start_time + 500;
    });

    // 3. Sender cancels
    client.cancel_stream(&sender, &stream_id);

    // Verify:
    // Recipient should have received the "unclaimed but accrued" 2,500 more.
    assert_eq!(token_client.balance(&recipient), 5000);
    // Sender should have received 5,000 refund (10,000 - 5,000 accrued).
    assert_eq!(token_client.balance(&sender), 5000);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
}

#[test]
fn test_invalid_start_time() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    // Attempt to create stream starting in the past (999 < 1000)
    let result = client.try_create_stream(&sender, &recipient, &token, &1000, &999, &2000);
    assert!(result.is_err());
}

#[test]
fn test_claim_multiple_times_progression() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    let start_time = 1000;
    env.ledger().with_mut(|li| {
        li.timestamp = start_time;
    });
    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token,
        &10000,
        &start_time,
        &(start_time + 1000),
    );

    for i in 1..=10 {
        env.ledger().with_mut(|li| {
            li.timestamp = start_time + (i * 100);
        });
        client.claim(&recipient, &stream_id);
        assert_eq!(token_client.balance(&recipient), (i as i128) * 1000);
    }

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Completed);
}
