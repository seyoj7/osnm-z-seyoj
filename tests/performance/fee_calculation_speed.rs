use std::time::Instant;

use alloy_primitives::U256;
use opensea_mint::domain::{AutomaticFeePolicy, Eip1559Fees};

#[test]
fn measures_automatic_fee_calculation_latency() {
    let policy = AutomaticFeePolicy::new(12_500, 11_250).expect("policy");
    let estimate = Eip1559Fees {
        max_fee_per_gas: U256::from(30_000_000_000_u64),         // 30 Gwei
        max_priority_fee_per_gas: U256::from(1_500_000_000_u64), // 1.5 Gwei
    };
    let iterations = 100_000;

    let start = Instant::now();
    for _ in 0..iterations {
        let initial = policy.initial(estimate).expect("initial");
        let _ = policy.replacement(initial).expect("replacement");
    }
    let elapsed = start.elapsed();

    let per_calc_ns = elapsed.as_nanos() as f64 / iterations as f64;
    eprintln!(
        "Fee calculation (initial + replacement): {} iterations in {:?} ({:.0}ns/pair)",
        iterations, elapsed, per_calc_ns
    );

    // Fee arithmetic is pure U256 math — should be sub-microsecond
    assert!(
        per_calc_ns < 10_000.0,
        "Fee calculation took {:.0}ns, expected <10µs",
        per_calc_ns
    );
}

#[test]
fn measures_maximum_fee_escalation_chain() {
    let policy = AutomaticFeePolicy::new(12_500, 11_250).expect("policy");
    let initial = Eip1559Fees {
        max_fee_per_gas: U256::from(30_000_000_000_u64),
        max_priority_fee_per_gas: U256::from(1_500_000_000_u64),
    };
    let max_attempts = 10;
    let iterations = 10_000;

    let start = Instant::now();
    for _ in 0..iterations {
        let mut fees = policy.initial(initial).expect("initial");
        for _ in 1..max_attempts {
            fees = policy.replacement(fees).expect("replacement");
        }
        // Prevent optimization
        assert!(fees.max_fee_per_gas > initial.max_fee_per_gas);
    }
    let elapsed = start.elapsed();

    let per_chain_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "Fee escalation chain (10 replacements): {} iterations in {:?} ({:.1}µs/chain)",
        iterations, elapsed, per_chain_us
    );

    assert!(per_chain_us < 100.0);
}

#[test]
fn fee_policy_boundary_values_are_correct() {
    // Verify the multiplier math at boundary values
    let policy = AutomaticFeePolicy::new(12_500, 11_250).expect("policy");

    // Zero fees should produce zero results
    let zero_fees = Eip1559Fees {
        max_fee_per_gas: U256::ZERO,
        max_priority_fee_per_gas: U256::ZERO,
    };
    let initial = policy.initial(zero_fees).expect("zero initial");
    assert_eq!(initial.max_fee_per_gas, U256::ZERO);

    // Very large fees should not overflow for reasonable values
    let large_fees = Eip1559Fees {
        max_fee_per_gas: U256::from(1_000_000_000_000_u64), // 1 TGwei
        max_priority_fee_per_gas: U256::from(100_000_000_000_u64),
    };
    let initial = policy.initial(large_fees).expect("large initial");
    assert!(initial.max_fee_per_gas > large_fees.max_fee_per_gas);

    // Invalid multiplier should be rejected
    assert!(AutomaticFeePolicy::new(9_999, 11_250).is_err()); // below 10000
    assert!(AutomaticFeePolicy::new(12_500, 10_000).is_err()); // replacement not > 10000
}
