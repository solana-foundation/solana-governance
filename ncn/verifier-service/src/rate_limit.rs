//! Rate-limit keying that is safe behind a reverse proxy.
//!
//! tower_governor's default `PeerIpKeyExtractor` keys buckets on the TCP peer socket. Behind a
//! proxy (e.g. Cloudflare) that peer is always the proxy, so every client collapses into one shared
//! bucket. [`TrustedProxyKeyExtractor`] instead keys on the real client IP carried in
//! `CF-Connecting-IP` / `X-Forwarded-For` / `X-Real-IP`, but only when the connecting peer is a
//! trusted proxy; otherwise it falls back to the peer IP so a directly-connecting attacker cannot
//! forge headers to mint unlimited buckets.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request};
use ipnet::IpNet;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;
use tracing::{info, warn};

use crate::database::constants::DEFAULT_DB_PATH;

const DEFAULT_CF_IPS_V4_URL: &str = "https://www.cloudflare.com/ips-v4";
const DEFAULT_CF_IPS_V6_URL: &str = "https://www.cloudflare.com/ips-v6";
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const CACHE_FILE_NAME: &str = "cloudflare-ips.cache";

/// Parse a list of CIDRs / bare IPs separated by commas or newlines.
///
/// Blank lines and `#` comments are ignored. Bare IPs are normalized to host nets (`/32`/`/128`).
/// Unparseable entries are skipped with a warning rather than failing the whole list.
pub fn parse_cidr_list(raw: &str) -> Vec<IpNet> {
    raw.split([',', '\n', '\r'])
        .map(str::trim)
        .filter(|t| !t.is_empty() && !t.starts_with('#'))
        .filter_map(|t| match t.parse::<IpNet>() {
            Ok(net) => Some(net),
            Err(_) => match t.parse::<IpAddr>() {
                Ok(ip) => Some(IpNet::from(ip)),
                Err(_) => {
                    warn!("Ignoring unparseable trusted-proxy entry: {t:?}");
                    None
                }
            },
        })
        .collect()
}

/// Resolve the client IP from forwarded headers: `CF-Connecting-IP` > first `X-Forwarded-For` >
/// `X-Real-IP`. Returns `None` if none are present or parseable.
fn client_ip_from_headers(headers: &HeaderMap) -> Option<IpAddr> {
    if let Some(ip) = headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
    {
        return Some(ip);
    }
    if let Some(ip) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
    {
        return Some(ip);
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
}

/// Where the fetched Cloudflare list is cached. Override with `TRUSTED_PROXY_CACHE_PATH`; defaults
/// to a file alongside the SQLite database (`DB_PATH`'s directory).
fn trusted_proxy_cache_path() -> PathBuf {
    if let Ok(p) = std::env::var("TRUSTED_PROXY_CACHE_PATH") {
        return PathBuf::from(p);
    }
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let dir = Path::new(&db_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    dir.join(CACHE_FILE_NAME)
}

/// Fetch Cloudflare's published IPv4 + IPv6 ranges and return them as one newline-joined blob.
async fn fetch_cloudflare_cidrs() -> anyhow::Result<String> {
    let v4_url = std::env::var("CLOUDFLARE_IPS_V4_URL")
        .unwrap_or_else(|_| DEFAULT_CF_IPS_V4_URL.to_string());
    let v6_url = std::env::var("CLOUDFLARE_IPS_V6_URL")
        .unwrap_or_else(|_| DEFAULT_CF_IPS_V6_URL.to_string());

    let client = reqwest::Client::builder().timeout(FETCH_TIMEOUT).build()?;
    let v4 = client
        .get(&v4_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let v6 = client
        .get(&v6_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Ok(format!("{v4}\n{v6}"))
}

/// Resolve the set of trusted proxy CIDRs at startup.
///
/// Precedence:
/// 1. `TRUSTED_PROXY_CIDRS` set to a literal list → use it verbatim (no network call). The empty
///    string or `none` disables forwarded-header trust (rate limiting keys on the peer IP).
/// 2. `TRUSTED_PROXY_CIDRS` unset or the keyword `cloudflare` → fetch Cloudflare's published ranges,
///    caching them to disk. On a transient fetch failure, fall back to the cached copy.
///
/// Fails closed: if Cloudflare ranges can be obtained from neither the network nor the cache, this
/// returns an error so the service never silently reverts to the shared-bucket behavior.
pub async fn load_trusted_proxy_cidrs() -> anyhow::Result<Vec<IpNet>> {
    if let Ok(raw) = std::env::var("TRUSTED_PROXY_CIDRS") {
        let trimmed = raw.trim();
        if !trimmed.eq_ignore_ascii_case("cloudflare") {
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                info!(
                    "TRUSTED_PROXY_CIDRS disables forwarded-header trust; rate limiting by peer IP"
                );
                return Ok(Vec::new());
            }
            let nets = parse_cidr_list(trimmed);
            anyhow::ensure!(
                !nets.is_empty(),
                "TRUSTED_PROXY_CIDRS={trimmed:?} did not parse to any valid CIDR/IP"
            );
            info!(
                "Loaded {} trusted proxy CIDR(s) from TRUSTED_PROXY_CIDRS",
                nets.len()
            );
            return Ok(nets);
        }
    }

    let cache_path = trusted_proxy_cache_path();
    let nets = match fetch_cloudflare_cidrs().await {
        Ok(text) => {
            let nets = parse_cidr_list(&text);
            if !nets.is_empty() {
                if let Err(e) = std::fs::write(&cache_path, &text) {
                    warn!(
                        "Failed to cache Cloudflare IP ranges to {}: {e}",
                        cache_path.display()
                    );
                }
                info!(
                    "Fetched {} Cloudflare proxy CIDR(s); cached to {}",
                    nets.len(),
                    cache_path.display()
                );
            }
            nets
        }
        Err(fetch_err) => match std::fs::read_to_string(&cache_path) {
            Ok(text) => {
                let nets = parse_cidr_list(&text);
                warn!(
                    "Could not fetch Cloudflare IP ranges ({fetch_err:#}); using {} cached CIDR(s) from {}",
                    nets.len(),
                    cache_path.display()
                );
                nets
            }
            Err(_) => Vec::new(),
        },
    };

    anyhow::ensure!(
        !nets.is_empty(),
        "Could not obtain Cloudflare IP ranges (fetch failed and no usable cache at {}). \
         Set TRUSTED_PROXY_CIDRS to your proxy's CIDR ranges, or 'none' to disable forwarded-header trust.",
        cache_path.display()
    );
    Ok(nets)
}

/// A [`KeyExtractor`] that keys on the real client IP when the connecting peer is a trusted proxy,
/// and on the peer IP otherwise. See the module docs for the threat model.
#[derive(Clone)]
pub struct TrustedProxyKeyExtractor {
    trusted: Arc<Vec<IpNet>>,
}

impl TrustedProxyKeyExtractor {
    pub fn new(trusted: Vec<IpNet>) -> Self {
        Self {
            trusted: Arc::new(trusted),
        }
    }

    fn is_trusted(&self, ip: IpAddr) -> bool {
        self.trusted.iter().any(|net| net.contains(&ip))
    }
}

impl KeyExtractor for TrustedProxyKeyExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        match req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip())
        {
            Some(peer_ip) => {
                if self.is_trusted(peer_ip) {
                    if let Some(client_ip) = client_ip_from_headers(req.headers()) {
                        return Ok(client_ip);
                    }
                }
                Ok(peer_ip)
            }
            // Guaranteed present in production via into_make_service_with_connect_info; fail closed.
            None => Err(GovernorError::UnableToExtractKey),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::ConnectInfo;
    use axum::http::Request;

    fn req_with(peer: &str, xff: Option<&str>, cf: Option<&str>) -> Request<()> {
        let mut builder = Request::builder();
        if let Some(v) = xff {
            builder = builder.header("x-forwarded-for", v);
        }
        if let Some(v) = cf {
            builder = builder.header("cf-connecting-ip", v);
        }
        let mut req = builder.body(()).unwrap();
        let sa: SocketAddr = format!("{peer}:1234").parse().unwrap();
        req.extensions_mut().insert(ConnectInfo(sa));
        req
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn trusted_peer_uses_forwarded_ip() {
        let ex = TrustedProxyKeyExtractor::new(parse_cidr_list("127.0.0.1/32"));
        let req = req_with("127.0.0.1", Some("203.0.113.7"), None);
        assert_eq!(ex.extract(&req).unwrap(), ip("203.0.113.7"));
    }

    #[test]
    fn trusted_peer_prefers_cf_connecting_ip() {
        let ex = TrustedProxyKeyExtractor::new(parse_cidr_list("127.0.0.1/32"));
        let req = req_with("127.0.0.1", Some("203.0.113.7"), Some("198.51.100.9"));
        assert_eq!(ex.extract(&req).unwrap(), ip("198.51.100.9"));
    }

    #[test]
    fn untrusted_peer_ignores_forged_headers() {
        let ex = TrustedProxyKeyExtractor::new(parse_cidr_list("10.0.0.0/8"));
        // Peer 127.0.0.1 is NOT in 10.0.0.0/8, so forged headers are ignored.
        let req = req_with("127.0.0.1", Some("203.0.113.7"), Some("198.51.100.9"));
        assert_eq!(ex.extract(&req).unwrap(), ip("127.0.0.1"));
    }

    #[test]
    fn trusted_peer_without_headers_falls_back_to_peer() {
        let ex = TrustedProxyKeyExtractor::new(parse_cidr_list("127.0.0.1/32"));
        let req = req_with("127.0.0.1", None, None);
        assert_eq!(ex.extract(&req).unwrap(), ip("127.0.0.1"));
    }

    #[test]
    fn empty_trusted_set_always_keys_on_peer() {
        let ex = TrustedProxyKeyExtractor::new(Vec::new());
        let req = req_with("127.0.0.1", Some("203.0.113.7"), None);
        assert_eq!(ex.extract(&req).unwrap(), ip("127.0.0.1"));
    }

    #[test]
    fn parse_cidr_list_handles_mixed_input() {
        let nets =
            parse_cidr_list("173.245.48.0/20, 1.1.1.1\n# a comment\n2400:cb00::/32\n\ngarbage");
        assert_eq!(nets.len(), 3);
        assert!(nets.iter().any(|n| n.contains(&ip("173.245.48.5"))));
        assert!(nets.iter().any(|n| n.contains(&ip("1.1.1.1"))));
        assert!(nets.iter().any(|n| n.contains(&ip("2400:cb00::1"))));
    }

    #[test]
    fn client_ip_priority_order() {
        let mut h = HeaderMap::new();
        h.insert(
            "x-forwarded-for",
            "203.0.113.1, 70.41.3.18".parse().unwrap(),
        );
        assert_eq!(client_ip_from_headers(&h), Some(ip("203.0.113.1")));
        // CF-Connecting-IP wins over X-Forwarded-For.
        h.insert("cf-connecting-ip", "198.51.100.2".parse().unwrap());
        assert_eq!(client_ip_from_headers(&h), Some(ip("198.51.100.2")));
    }
}
