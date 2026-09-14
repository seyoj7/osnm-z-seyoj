use std::time::Instant;

use alloy_primitives::{Bytes, U256};
use opensea_mint::signing::WalletSigner;
use opensea_mint::transaction::{Eip1559Transaction, sign_eip1559_transaction};

const TEST_KEY: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";

fn sample_eip1559_transaction() -> Eip1559Transaction {
    Eip1559Transaction {
        chain_id: 8453,
        nonce: 7,
        max_priority_fee_per_gas: U256::from(1_000_000_u64),
        max_fee_per_gas: U256::from(2_000_000_u64),
        gas_limit: 300_000,
        target: "0x00005EA00Ac477B1030CE78506496e8C2dE24bf5"
            .parse()
            .expect("target"),
        value: U256::from(50_000_000_000_000_u64),
        calldata: Bytes::from(vec![0x4b, 0x61, 0xcd, 0x6f, 0xde, 0xad, 0xbe, 0xef]),
    }
}

#[test]
fn measures_single_transaction_signing_latency() {
    let signer = WalletSigner::from_private_key(TEST_KEY).expect("signer");
    let transaction = sample_eip1559_transaction();

    // Warm up
    let _ = sign_eip1559_transaction(&transaction, &signer).expect("warmup");

    let start = Instant::now();
    let signed = sign_eip1559_transaction(&transaction, &signer).expect("signed");
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 50,
        "Single transaction signing took {}ms, expected <50ms",
        elapsed.as_millis()
    );
    assert_eq!(signed.raw()[0], 2); // EIP-1559 type
}

#[test]
fn measures_batch_transaction_signing_throughput() {
    let signer = WalletSigner::from_private_key(TEST_KEY).expect("signer");
    let transaction = sample_eip1559_transaction();
    let iterations = 1_000;

    // Warm up
    for _ in 0..10 {
        let _ = sign_eip1559_transaction(&transaction, &signer).expect("warmup");
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = sign_eip1559_transaction(&transaction, &signer).expect("signed");
    }
    let elapsed = start.elapsed();

    let per_sign_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "Transaction signing throughput: {} iterations in {:?} ({:.1}µs/sign, {:.0} signs/sec)",
        iterations,
        elapsed,
        per_sign_us,
        1_000_000.0 / per_sign_us
    );

    // k256 ECDSA should comfortably handle >1000 signs/sec on any modern CPU
    assert!(
        per_sign_us < 10_000.0,
        "Per-sign latency {:.1}µs exceeds 10ms threshold",
        per_sign_us
    );
}

#[test]
fn measures_wallet_signer_creation_latency() {
    let iterations = 100;

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = WalletSigner::from_private_key(TEST_KEY).expect("signer");
    }
    let elapsed = start.elapsed();

    let per_creation_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "Signer creation: {} iterations in {:?} ({:.1}µs/create)",
        iterations,
        elapsed,
        per_creation_us
    );

    assert!(
        per_creation_us < 5_000.0,
        "Per-creation latency {:.1}µs exceeds 5ms threshold",
        per_creation_us
    );
}

#[test]
fn measures_personal_message_signing_throughput() {
    let signer = WalletSigner::from_private_key(TEST_KEY).expect("signer");
    let message = b"Click to sign in and accept the OpenSea Terms of Service";
    let iterations = 1_000;

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = signer.sign_personal_message(message).expect("signed");
    }
    let elapsed = start.elapsed();

    let per_sign_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "Personal message signing: {} iterations in {:?} ({:.1}µs/sign)",
        iterations,
        elapsed,
        per_sign_us
    );

    assert!(per_sign_us < 10_000.0);
}
