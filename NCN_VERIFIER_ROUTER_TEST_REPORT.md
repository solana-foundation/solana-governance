# NCN Verifier and Router Reliability Test Report

## 1. Conclusion

No verifier timed out or returned incorrect proof data during the direct test.
All 11 verifiers returned the same valid proof.

The tests found these availability problems:

- The production router timed out twice before it selected a verifier.
- Digital Energy returned one HTTP `429` response during the steady test.
- Cross-network tests indicate shared rate-limit buckets at Ha1iad3, Titan Analytics, and Digital Energy.
- The router entries for Exo Tech and Solgov do not work from an HTTPS web application.

The clients do not retry a failed request.
Thus, one router, rate-limit, or web-route failure can stop a vote.

The smallest client correction is a maximum of three attempts.
Operators must also correct the proxy configurations and the HTTPS routes.

## 2. Terms

**Router** means `https://ncn-governance.solana.com`.

**Verifier** means a service that supplies a Merkle proof.

**Direct test** means a request sent directly to one verifier.

**Steady test** means a request every two seconds through the router.

**Stress test** means many requests in a short time.

## 3. Test configuration

The tests used the Solana mainnet network.

The tests used snapshot slot `440641000`.

The tests used one known vote account from that snapshot.

The tests ran from an Amsterdam validator host and a second network.
The two test sources had different public IP addresses.

A successful proof response had to meet all these conditions:

- The HTTP status was `200`.
- The response was valid JSON.
- The response contained a meta Merkle leaf.
- The response contained a Merkle proof array.
- The canonical JSON hash matched the reference hash.

The reference SHA-256 hash was:

```text
3a9a5aa1da6578c48bbc00567e3960e6e30e36d7dde779490071c40f684b9dcf
```

The proof contained 10 hashes.
The proof reported `252716916209335` active stake lamports.

## 4. Direct verifier test

The first direct request to each verifier returned HTTP `200`.
All 11 verifiers returned the same canonical proof hash.
No direct verifier timed out during this test.

| Verifier | Service version | Build | Direct result |
|---|---:|---:|---|
| Ha1iad3 | `0.5.0-40200` | `65ba35cf9ed9` | HTTP 200, valid proof |
| Lantern | `0.4.0-40100` | `2cc40baa9917` | HTTP 200, valid proof |
| Titan Analytics | `0.5.0-40200` | `95b4960ddeaa` | HTTP 200, valid proof |
| Adra Finance / Solgov | `0.5.0-40200` | `304f47f88a71` | HTTP 200, valid proof |
| Blocksize | `0.3.0-40000` | `663519a9027e` | HTTP 200, valid proof |
| Digital Energy | `0.5.0-40200` | `65ba35cf9ed9` | HTTP 200, valid proof |
| Stakeware | `0.5.0-40200` | `65ba35cf9ed9` | HTTP 200, valid proof |
| Prompt Logic | `0.4.0-40100` | `fdde0543eaf3` | HTTP 200, valid proof |
| Exo Tech | `0.5.0-40200` | `95b4960ddeaa` | HTTP 200, valid proof |
| Chainflow | `0.4.0-40100` | `2cc40baa9917` | HTTP 200, valid proof |
| Brewlabs | `0.5.0-40200` | `65ba35cf9ed9` | HTTP 200, valid proof |

## 5. Steady router test

The steady test sent 66 proof requests through the router.
The test sent one request every two seconds.

The router sent 64 requests to verifiers.
Of these requests, 63 returned the correct proof.
Digital Energy returned one HTTP `429` response.

Two requests timed out before the router selected a verifier.

| Target | Requests | Successful proofs | Errors |
|---|---:|---:|---:|
| Router, before redirect | 2 | 0 | 2 timeouts |
| Ha1iad3 | 3 | 3 | 0 |
| Lantern | 9 | 9 | 0 |
| Titan Analytics | 6 | 6 | 0 |
| Adra Finance / Solgov | 8 | 8 | 0 |
| Blocksize | 8 | 8 | 0 |
| Digital Energy | 7 | 6 | 1 HTTP 429 |
| Stakeware | 4 | 4 | 0 |
| Prompt Logic | 3 | 3 | 0 |
| Exo Tech | 2 | 2 | 0 |
| Chainflow | 5 | 5 | 0 |
| Brewlabs | 9 | 9 | 0 |

The router timeouts occurred at these UTC times:

- `2026-08-23T13:35:30Z`
- `2026-08-23T13:35:51Z`

Both requests reached Cloudflare address `104.20.41.212`.
Neither request received an HTTP response in six seconds.
Neither request received a redirect.

The production router uses redirect mode.
In this mode, the router does not contact the selected verifier.
Thus, these two timeouts occurred before verifier selection.

The Solana Foundation must examine the router and Cloudflare logs at these times.

## 6. Rate-limit test

The stress test sent 150 requests in a short time.
A 60-request test started immediately after the stress test.

The second test received 30 HTTP `429` responses from verifier services.
Ten of the 11 verifiers returned at least one HTTP `429` response.

These values are not normal availability values.
The stress test intentionally used the rate-limit tokens.
The results show the rate-limit behavior.

The default global limit has these values:

- Burst size: 10 requests.
- Refill interval: one token every 10 seconds.

The public documentation calls this limit `10 req/s`.
This description does not agree with the code or the library behavior.

If all users share one bucket, they get only 10 initial requests.
After this burst, all users share one request every 10 seconds.

## 7. Shared proxy bucket evidence

The Amsterdam test source first used the rate-limit buckets.
The second source then sent one direct request to each verifier.

The second source immediately received HTTP `429` from these verifiers:

- Ha1iad3
- Titan Analytics
- Digital Energy

The second source had a different public IP address.
This result is consistent with a shared reverse-proxy bucket.

All three services use Nginx.
Their service versions include the trusted-proxy rate-limit correction.

Ha1iad3 and Digital Energy use the same build as Brewlabs and Stakeware.
Titan Analytics uses the same build as Exo Tech.
Thus, the binary version does not explain the different behavior.

The probable cause is verifier deployment configuration.
The verifier must trust the IP range of its direct reverse proxy.
The reverse proxy must append the client address to `X-Forwarded-For`.

If the peer is not trusted, the verifier uses the peer address as the key.
In an Nginx or Docker deployment, this peer can be the same for all users.

The operator must verify `TRUSTED_PROXY_CIDRS` against the actual network path.
The operator must also verify the Nginx forwarded-address configuration.

## 8. Browser test

A Chrome test ran from an HTTPS page.

The Exo Tech proof URL uses plain HTTP.
Chrome blocked this request as mixed content.

The Solgov proof URL returned a redirect without the required CORS header.
The redirect also specified an HTTP destination.
Chrome blocked the request.

The direct `https://www.solgov.com` proof URL returned HTTP `200` in Chrome.

Thus, the CLI can use these routes while the web application cannot use them.

The router must not select Exo Tech until Exo Tech supplies HTTPS.
The Solgov router entry must use `https://www.solgov.com`.
Solgov must not redirect an HTTPS proof request to HTTP.

## 9. Failure mechanism

The router selects one verifier at random.
The router then returns one HTTP `302` redirect.

The router does not observe the final verifier response in redirect mode.
The router cannot select a second verifier after a failure.

The router updates its whitelist every two hours.
It uses the verifier `/meta` response during this update.
A verifier can fail after the update and remain in the routing pool.

The CLI sends one proof request.
The CLI has a 15-second timeout.
The CLI does not retry a transient failure.

The web application also sends one proof request.
It does not retry a transient failure.

Thus, one bad selection stops the vote operation.

## 10. Required operational actions

### 10.1 Router operator

1. Examine the router and Cloudflare logs at the two UTC times in Section 5.
2. Check CPU use, file descriptor use, thread use, and origin connection errors.
3. Remove Exo Tech from the pool until it supplies HTTPS.
4. Change the Solgov entry to `https://www.solgov.com`.

### 10.2 Verifier operators

1. Identify the direct peer address that the verifier service receives.
2. Add only the required peer ranges to `TRUSTED_PROXY_CIDRS`.
3. Configure Nginx to append the client address to `X-Forwarded-For`.
4. Confirm that two different client IP addresses use different rate-limit buckets.
5. Monitor HTTP `429` responses.

## 11. Recommended code changes

### 11.1 Client retry

Add a maximum of three attempts to the CLI and the web application.

Retry these failures:

- Network and timeout errors.
- HTTP `429`.
- HTTP `502`, `503`, and `504`.

Each new router request can select a different verifier.
Do not retry permanent input errors.

Include the final verifier URL in each error message.
This information identifies the selected verifier.

### 11.2 Deployment configuration

Add `TRUSTED_PROXY_CIDRS` to the verifier setup script.
Do not require operators to edit the generated Docker command.

Correct the rate-limit documentation.
State that the value is a refill interval in seconds.

Add a metric for HTTP `429` responses.
The current administrative statistics do not count these responses.

### 11.3 Router eligibility

Require HTTPS for all verifier domains.
Reject a redirect chain that changes from HTTPS to HTTP.

Do not add a complex circuit breaker as the first correction.
First add client retries and correct the operator configurations.

## 12. Test limitations

The router selects verifiers at random.
Thus, the number of requests to each verifier was different.

The sample size does not define a service-level objective.
It identifies reproducible failure modes.

The stress test temporarily used the rate-limit tokens.
The tokens refill automatically.

The tests used HTTP GET requests only.
The tests did not change on-chain or verifier data.

External tests cannot identify the internal cause of a router stall.
The router operator must use the server and Cloudflare logs.

## 13. References

- [NCN router selection and redirect code](https://github.com/solana-foundation/solana-governance/blob/main/ncn-router/src/router.rs)
- [Verifier rate-limit configuration](https://github.com/solana-foundation/solana-governance/blob/main/ncn/verifier-service/src/main.rs)
- [Trusted proxy key extraction](https://github.com/solana-foundation/solana-governance/blob/main/ncn/verifier-service/src/rate_limit.rs)
- [Trusted proxy correction, PR 89](https://github.com/solana-foundation/solana-governance/pull/89)
- [Verifier deployment script](https://github.com/solana-foundation/solana-governance/blob/main/ncn/verifier-service/src/scripts/setup.sh)
- [CLI proof request code](https://github.com/solana-foundation/solana-governance/blob/main/svmgov/cli/src/utils/api_helpers.rs)
- [Web proof request code](https://github.com/solana-foundation/solana-governance/blob/main/frontend/src/chain/instructions/helpers.ts)
- [Draft HTTPS enforcement change, PR 88](https://github.com/solana-foundation/solana-governance/pull/88)
