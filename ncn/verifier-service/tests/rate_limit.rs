//! Regression tests for SOLA5-7: rate-limit buckets must key on the real client IP behind a
//! trusted proxy, and must NOT be forgeable by an untrusted peer setting forwarded headers.
//!
//! The server binds loopback, so the TCP peer is always 127.0.0.1. Each test sets
//! `TRUSTED_PROXY_CIDRS` to decide whether that peer is trusted, and uses a small `GLOBAL_RATE_BURST`
//! with a long refill so token accounting is deterministic within the test.

mod common;

use common::setup_server_with_env;
use reqwest::{Client, StatusCode};
use solana_sdk::signature::Keypair;

/// When the peer is a trusted proxy, distinct forwarded clients get independent buckets: one client
/// exhausting its burst must not rate-limit an unrelated client. This is the per-user isolation that
/// the original peer-IP keying destroyed.
#[tokio::test]
#[serial_test::serial]
async fn trusted_proxy_separates_forwarded_clients() -> anyhow::Result<()> {
    let keypair = Keypair::new();
    let (base_url, _guard) = setup_server_with_env(
        &keypair,
        &[
            ("TRUSTED_PROXY_CIDRS", "127.0.0.1/32"), // loopback peer IS trusted
            ("GLOBAL_RATE_BURST", "2"),
            ("GLOBAL_REFILL_INTERVAL", "3600"), // no refill during the test
        ],
    )
    .await?;

    let client = Client::new();
    let healthz = format!("{}/healthz", base_url);

    // Client A burns its 2-request burst (own bucket, keyed on its forwarded IP).
    let a1 = client
        .get(&healthz)
        .header("x-forwarded-for", "203.0.113.10")
        .send()
        .await?;
    let a2 = client
        .get(&healthz)
        .header("x-forwarded-for", "203.0.113.10")
        .send()
        .await?;
    let a3 = client
        .get(&healthz)
        .header("x-forwarded-for", "203.0.113.10")
        .send()
        .await?;

    // Client B has its own bucket and must be unaffected by A's exhaustion.
    let b1 = client
        .get(&healthz)
        .header("x-forwarded-for", "198.51.100.20")
        .send()
        .await?;

    assert_eq!(a1.status(), StatusCode::OK);
    assert_eq!(a2.status(), StatusCode::OK);
    assert_eq!(
        a3.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "A should be limited after its burst"
    );
    assert_eq!(
        b1.status(),
        StatusCode::OK,
        "unrelated client B must not share A's bucket"
    );

    Ok(())
}

/// When the peer is NOT a trusted proxy, forwarded headers are ignored and every request keys on the
/// peer IP. A directly-connecting attacker therefore cannot mint a fresh bucket per forged IP: a
/// brand-new forged `X-Forwarded-For` is still rejected once the shared peer bucket is exhausted.
#[tokio::test]
#[serial_test::serial]
async fn untrusted_peer_cannot_forge_separate_buckets() -> anyhow::Result<()> {
    let keypair = Keypair::new();
    let (base_url, _guard) = setup_server_with_env(
        &keypair,
        &[
            ("TRUSTED_PROXY_CIDRS", "10.0.0.0/8"), // loopback peer is NOT in this range
            ("GLOBAL_RATE_BURST", "2"),
            ("GLOBAL_REFILL_INTERVAL", "3600"),
        ],
    )
    .await?;

    let client = Client::new();
    let healthz = format!("{}/healthz", base_url);

    // Each request carries a unique, never-before-seen forged client IP. If forged headers minted
    // per-IP buckets, all of these would succeed. Because the untrusted peer is keyed instead, the
    // shared bucket (capacity 2) is quickly exhausted.
    let mut statuses = Vec::new();
    for i in 0..8 {
        let forged = format!("203.0.113.{}", 100 + i);
        let resp = client
            .get(&healthz)
            .header("x-forwarded-for", forged)
            .send()
            .await?;
        statuses.push(resp.status());
    }

    let ok_count = statuses.iter().filter(|s| **s == StatusCode::OK).count();
    assert!(
        ok_count <= 2,
        "forged forwarded IPs must share one peer bucket (burst 2), but {ok_count} succeeded: {statuses:?}",
    );
    assert_eq!(
        *statuses.last().unwrap(),
        StatusCode::TOO_MANY_REQUESTS,
        "a fresh forged IP must still be rate-limited: {statuses:?}",
    );

    Ok(())
}
