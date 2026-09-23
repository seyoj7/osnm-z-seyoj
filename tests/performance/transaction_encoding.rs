use std::time::Instant;

use alloy_primitives::{Bytes, U256};
use opensea_mint::{
    signing::WalletSigner,
    transaction::{Eip1559Transaction, sign_eip1559_transaction},
};

const TEST_KEY: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";

fn realistic_mint_transaction(calldata_size: usize) -> Eip1559Transaction {
    Eip1559Transaction {
        chain_id: 8453,
        nonce: 42,
        max_priority_fee_per_gas: U256::from(50_000_000_u64),       // 0.05 Gwei
        max_fee_per_gas: U256::from(1_000_000_000_u64),             // 1 Gwei
        gas_limit: 300_000,
        target: "0x00005EA00Ac477B1030CE78506496e8C2dE24bf5"
            .parse()
            .expect("target"),
        value: U256::from(80_000_000_000_000_000_u64), // 0.08 ETH
        calldata: Bytes::from(vec![0xab; calldata_size]),
    }
}

#[test]
fn measures_transaction_encoding_with_varying_calldata_sizes() {
    let signer = WalletSigner::from_private_key(TEST_KEY).expect("signer");
    let sizes = [4, 68, 256, 1024, 4096, 16384];
    let iterations = 500;

    for &size in &sizes {
        let transaction = realistic_mint_transaction(size);

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = sign_eip1559_transaction(&transaction, &signer).expect("signed");
        }
        let elapsed = start.elapsed();

        let per_encode_us = elapsed.as_micros() as f64 / iterations as f64;
        eprintln!(
            "TX encoding+signing (calldata={}B): {:.1}µs/tx ({:.0} tx/sec)",
            size,
            per_encode_us,
            1_000_000.0 / per_encode_us
        );

        // Debug builds are ~10-20x slower; 50ms accommodates unoptimized ECDSA
        assert!(
            per_encode_us < 50_000.0,
            "calldata={}B took {:.1}µs, expected <50ms (debug build)",
            size,
            per_encode_us
        );
    }
}

#[test]
fn measures_raw_rlp_encoding_overhead() {
    let signer = WalletSigner::from_private_key(TEST_KEY).expect("signer");
    // Minimal calldata = pure overhead measurement
    let minimal = realistic_mint_transaction(4);
    // Large calldata = RLP + keccak cost
    let large = realistic_mint_transaction(16384);
    let iterations = 1_000;

    let start_minimal = Instant::now();
    for _ in 0..iterations {
        let _ = sign_eip1559_transaction(&minimal, &signer).expect("signed");
    }
    let elapsed_minimal = start_minimal.elapsed();

    let start_large = Instant::now();
    for _ in 0..iterations {
        let _ = sign_eip1559_transaction(&large, &signer).expect("signed");
    }
    let elapsed_large = start_large.elapsed();

    let overhead_per_tx_us = (elapsed_large.as_micros() as f64 - elapsed_minimal.as_micros() as f64)
        / iterations as f64;
    eprintln!(
        "RLP+keccak overhead for 16KB vs 4B calldata: {:.1}µs/tx",
        overhead_per_tx_us
    );

    // Debug builds inflate keccak + RLP cost significantly
    assert!(
        overhead_per_tx_us < 15_000.0,
        "RLP overhead {:.1}µs seems excessive for debug build",
        overhead_per_tx_us
    );
}

#[test]
fn signed_transaction_hash_is_deterministic() {
    let signer = WalletSigner::from_private_key(TEST_KEY).expect("signer");
    let transaction = realistic_mint_transaction(68);

    let signed_a = sign_eip1559_transaction(&transaction, &signer).expect("signed a");
    let signed_b = sign_eip1559_transaction(&transaction, &signer).expect("signed b");

    // k256 ECDSA with RFC 6979 is deterministic
    assert_eq!(signed_a.hash(), signed_b.hash());
    assert_eq!(signed_a.raw(), signed_b.raw());
}

#[test]
fn different_nonces_produce_different_hashes() {
    let signer = WalletSigner::from_private_key(TEST_KEY).expect("signer");
    let mut tx_a = realistic_mint_transaction(68);
    let mut tx_b = realistic_mint_transaction(68);
    tx_a.nonce = 0;
    tx_b.nonce = 1;

    let signed_a = sign_eip1559_transaction(&tx_a, &signer).expect("signed a");
    let signed_b = sign_eip1559_transaction(&tx_b, &signer).expect("signed b");

    assert_ne!(signed_a.hash(), signed_b.hash());
}
