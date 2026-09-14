use std::time::Instant;

use alloy_primitives::{Address, U256};
use opensea_mint::nft::{MintedAsset, encode_safe_transfer};

#[test]
fn measures_erc721_safe_transfer_encoding_throughput() {
    let from = Address::repeat_byte(0x11);
    let recipient = Address::repeat_byte(0x22);
    let asset = MintedAsset::Erc721 {
        token_id: U256::from(42),
    };
    let iterations = 10_000;

    let start = Instant::now();
    for _ in 0..iterations {
        let calldata = encode_safe_transfer(&asset, from, recipient);
        assert!(!calldata.is_empty());
    }
    let elapsed = start.elapsed();

    let per_encode_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "ERC-721 safeTransferFrom encoding: {} iterations in {:?} ({:.1}µs/encode)",
        iterations, elapsed, per_encode_us
    );

    assert!(per_encode_us < 100.0);
}

#[test]
fn measures_erc1155_safe_transfer_encoding_throughput() {
    let from = Address::repeat_byte(0x11);
    let recipient = Address::repeat_byte(0x22);
    let asset = MintedAsset::Erc1155 {
        token_id: U256::from(7),
        units: U256::from(3),
    };
    let iterations = 10_000;

    let start = Instant::now();
    for _ in 0..iterations {
        let calldata = encode_safe_transfer(&asset, from, recipient);
        assert!(!calldata.is_empty());
    }
    let elapsed = start.elapsed();

    let per_encode_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "ERC-1155 safeTransferFrom encoding: {} iterations in {:?} ({:.1}µs/encode)",
        iterations, elapsed, per_encode_us
    );

    assert!(per_encode_us < 100.0);
}

#[test]
fn transfer_encoding_produces_correct_selectors() {
    let from = Address::repeat_byte(1);
    let to = Address::repeat_byte(2);

    let erc721 = encode_safe_transfer(
        &MintedAsset::Erc721 {
            token_id: U256::from(9),
        },
        from,
        to,
    );
    // ERC-721 safeTransferFrom(address,address,uint256)
    assert_eq!(&erc721[..4], &[0x42, 0x84, 0x2e, 0x0e]);

    let erc1155 = encode_safe_transfer(
        &MintedAsset::Erc1155 {
            token_id: U256::from(9),
            units: U256::from(1),
        },
        from,
        to,
    );
    // ERC-1155 safeTransferFrom(address,address,uint256,uint256,bytes)
    assert_eq!(&erc1155[..4], &[0xf2, 0x42, 0x43, 0x2a]);
}

#[test]
fn measures_worst_case_multi_nft_forwarding() {
    // Simulate forwarding 10 ERC-721 NFTs (worst case for self-funded multi-wallet)
    let from = Address::repeat_byte(0x11);
    let recipient = Address::repeat_byte(0x22);
    let nft_count = 10;
    let iterations = 1_000;

    let assets: Vec<MintedAsset> = (0..nft_count)
        .map(|i| MintedAsset::Erc721 {
            token_id: U256::from(i),
        })
        .collect();

    let start = Instant::now();
    for _ in 0..iterations {
        for asset in &assets {
            let calldata = encode_safe_transfer(asset, from, recipient);
            assert!(!calldata.is_empty());
        }
    }
    let elapsed = start.elapsed();

    let per_batch_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "Encoding {} ERC-721 transfers: {:.1}µs/batch ({:.1}µs/transfer)",
        nft_count,
        per_batch_us,
        per_batch_us / nft_count as f64
    );

    // 10 transfers should still be well under 1ms
    assert!(
        per_batch_us < 1_000.0,
        "Batch encoding took {:.1}µs, expected <1ms",
        per_batch_us
    );
}
