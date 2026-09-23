use std::time::Instant;

use alloy_primitives::{Address, B256, Bytes, U256};
use opensea_mint::{
    signing::WalletSigner,
    sponsored::{
        SponsoredMintOperation, UnsignedSponsoredMintOperation,
        encode_execute_batch, sign_delegation, sign_operation,
        sponsored_outer_gas_limit, sponsored_outer_gas_limit_upper_bound,
        sponsored_setup_gas_limit,
    },
};

const SPONSOR_KEY: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";
const WALLET_KEY_2: &str = "0x0000000000000000000000000000000000000000000000000000000000000002";
const WALLET_KEY_3: &str = "0x0000000000000000000000000000000000000000000000000000000000000003";

fn test_operation(wallet: Address) -> UnsignedSponsoredMintOperation {
    UnsignedSponsoredMintOperation {
        wallet,
        mint_target: Address::repeat_byte(0x22),
        nft_contract: Address::repeat_byte(0x33),
        recipient: Address::repeat_byte(0x44),
        mint_value: U256::from(80_000_000_000_000_000_u64),
        expected_units: U256::ONE,
        mint_gas_limit: 300_000,
        wallet_gas_limit: 550_000,
        deadline: 2_000_000_000,
        mint_calldata: Bytes::from(vec![0x4b, 0x61, 0xcd, 0x6f]),
    }
}

fn distinct_wallet(index: u8) -> Address {
    // Each wallet gets a unique address by using the index as the repeat byte
    // Index must be > 0 and unique to avoid collisions with other test addresses
    Address::repeat_byte(index.wrapping_add(0x50))
}

#[test]
fn measures_delegation_signing_throughput() {
    let signer = WalletSigner::from_private_key(WALLET_KEY_2).expect("signer");
    let executor = Address::repeat_byte(0x11);
    let iterations = 500_u64;

    let start = Instant::now();
    for nonce in 0..iterations {
        let _ = sign_delegation(8453, executor, nonce, &signer).expect("delegation");
    }
    let elapsed = start.elapsed();

    let per_deleg_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "EIP-7702 delegation signing: {} iterations in {:?} ({:.1}µs/delegation)",
        iterations, elapsed, per_deleg_us
    );

    assert!(per_deleg_us < 15_000.0);
}

#[test]
fn measures_eip712_operation_signing_throughput() {
    let signer = WalletSigner::from_private_key(WALLET_KEY_3).expect("signer");
    let wallet = signer.identity().address;
    let sponsor = WalletSigner::from_private_key(SPONSOR_KEY)
        .expect("sponsor")
        .identity()
        .address;
    let dispatcher = Address::repeat_byte(0x11);
    let unsigned = test_operation(wallet);
    let batch_id = B256::repeat_byte(0xab);
    let iterations = 500;

    let start = Instant::now();
    for _ in 0..iterations {
        let mut operation = SponsoredMintOperation::unsigned(unsigned.clone());
        sign_operation(
            8453,
            dispatcher,
            sponsor,
            batch_id,
            0,
            &mut operation,
            &signer,
        )
        .expect("signed operation");
    }
    let elapsed = start.elapsed();

    let per_op_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "EIP-712 operation signing: {} iterations in {:?} ({:.1}µs/operation)",
        iterations, elapsed, per_op_us
    );

    assert!(per_op_us < 10_000.0);
}

#[test]
fn measures_batch_encoding_scalability() {
    // Test encoding cost as batch size scales from 1 to 25 wallets
    let batch_sizes = [1_usize, 5, 10, 15, 20, 25];
    let iterations = 100;
    let sponsor = WalletSigner::from_private_key(SPONSOR_KEY)
        .expect("sponsor")
        .identity()
        .address;
    let dispatcher = Address::repeat_byte(0x11);

    for &batch_size in &batch_sizes {
        let operations: Vec<SponsoredMintOperation> = (0..batch_size)
            .map(|i| {
                let wallet = distinct_wallet(i as u8);
                SponsoredMintOperation::unsigned(test_operation(wallet))
            })
            .collect();

        let batch_id = B256::repeat_byte(0xab);

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = encode_execute_batch(8453, dispatcher, sponsor, batch_id, &operations)
                .expect("encode");
        }
        let elapsed = start.elapsed();

        let per_encode_us = elapsed.as_micros() as f64 / iterations as f64;
        let calldata =
            encode_execute_batch(8453, dispatcher, sponsor, batch_id, &operations).expect("encode");
        eprintln!(
            "Batch encoding ({} wallets): {:.1}µs/encode, calldata={}B",
            batch_size,
            per_encode_us,
            calldata.len()
        );

        // Even 25-wallet batch should encode in under 1ms
        assert!(
            per_encode_us < 1_000.0,
            "Batch encoding ({} wallets) took {:.1}µs, expected <1ms",
            batch_size,
            per_encode_us
        );
    }
}

#[test]
fn measures_gas_limit_calculation_speed() {
    let iterations = 100_000;
    let batch_sizes = [1_usize, 10, 25];

    for &batch_size in &batch_sizes {
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = sponsored_setup_gas_limit(550_000, batch_size).expect("gas limit");
        }
        let elapsed = start.elapsed();

        let per_calc_ns = elapsed.as_nanos() as f64 / iterations as f64;
        eprintln!(
            "Setup gas limit calculation ({} wallets): {:.0}ns/calc",
            batch_size, per_calc_ns
        );

        assert!(per_calc_ns < 10_000.0); // sub-10-microsecond
    }
}

#[test]
fn gas_upper_bound_covers_actual_gas_limit() {
    // The upper bound must always be >= the actual gas limit for any valid batch
    let sponsor = WalletSigner::from_private_key(SPONSOR_KEY)
        .expect("sponsor")
        .identity()
        .address;
    let dispatcher = Address::repeat_byte(0x11);
    let batch_id = B256::repeat_byte(0xab);

    for batch_size in 1..=25_usize {
        let operations: Vec<SponsoredMintOperation> = (0..batch_size)
            .map(|i| SponsoredMintOperation::unsigned(test_operation(distinct_wallet(i as u8))))
            .collect();
        let calldata =
            encode_execute_batch(8453, dispatcher, sponsor, batch_id, &operations).expect("encode");
        let upper =
            sponsored_outer_gas_limit_upper_bound(calldata.len(), &operations).expect("upper");
        let actual = sponsored_outer_gas_limit(&calldata, &operations).expect("actual");
        assert!(
            upper >= actual,
            "Upper bound {} < actual {} for batch_size={}",
            upper,
            actual,
            batch_size
        );
    }
}
