# Local Modifications to OSNM-Z

This document summarizes all the uncommitted local modifications made to the original `zunmax/osnm-z` base project in this workspace.

## 1. Timing and Retry Optimizations (Performance Tuning)
Aggressively lowered the default timeout and delay settings to make the bot faster and more responsive, updating both `src/config.rs` and `.env.example`:
*   **Polling Delays:** Reduced `RECEIPT_POLL_BASE_DELAY_MS` from 250ms to **100ms** and `RECEIPT_POLL_MAX_DELAY_MS` from 2000ms to **500ms**.
*   **Request Timeouts:** Halved the general OpenSea timeout from 10s to **5s**, and reduced eligibility check timeout from 5s to **3s**.
*   **Retry Interval:** Reduced the fixed delay between retryable OpenSea requests from 250ms to **100ms**.
*   **Pending Timeout:** Lowered the time before a pending transaction is eligible for replacement from 20s to **12s**.

## 2. Connection Warmup (Latency Reduction)
In `src/command.rs`, a new "connection warmup" phase was added:
*   The bot now deliberately warms up the HTTP/2 connection **5 seconds** before the critical "calldata hot path".
*   It sends a lightweight metadata fetch (`collection_metadata`) to keep the OpenSea connection alive so that TLS handshake and ALPN negotiation latency is bypassed when speed actually matters.

## 3. Multi-Mint Gas Calculation Fix
In `src/multi_mint.rs`, the logic for how self-funded wallets calculate fees when safely transferring minted NFTs was optimized:
*   Instead of fetching new submission inputs from the gateway for every asset, the code now dynamically fetches the `latest_block`'s `base_fee_per_gas` and calculates gas locally.
*   It now pre-calculates the transfer nonces incrementally without redundant network lookups, improving transaction throughput.

## 4. Unified Snapshot Batching
In `src/chain.rs`, a new `decode_wallet_snapshot_batch` function was introduced along with robust test cases. 
*   This enables batching RPC node queries (like fetching balances, nonces, and current block base fee data) into unified JSON-RPC batch requests rather than independent calls.

## 5. New Performance Testing Suite
A comprehensive new benchmarking and performance testing module was built from scratch to measure and ensure the speed of critical local operations. The following untracked files were added:
*   `tests/performance.rs`
*   `tests/performance/fee_calculation_speed.rs`
*   `tests/performance/nft_transfer_encoding.rs`
*   `tests/performance/signing_throughput.rs`
*   `tests/performance/sponsored_batch_encoding.rs`
*   `tests/performance/transaction_encoding.rs`

This suite specifically tests fee calculation speed, signing throughput, raw EVM calldata encoding, EIP-7702 batch payload constructions, and NFT safe-transfer encoding.
