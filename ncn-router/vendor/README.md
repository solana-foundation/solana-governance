# Legacy TLS trust roots

These are the MPL-2.0 `webpki-roots` crates from crates.io, versions 0.24.0
and 0.25.4. Their source and licenses are retained, with the following changes:

- Remove the `emSign ECC Root CA - G3` and `AAA Certificate Services` trust
  anchors (the latter is labeled `Comodo AAA Services root` in 0.24.0).
- Omit 0.25.4's upstream development dependencies; its upstream tests and
  generation tools are not vendored.

These roots can authenticate the compromised eMudhra intermediates described
at <https://emudhra.we-have-a-problem.com/>. Solana/Anchor still pull rustls
0.21 and these legacy root bundles, which have no upstream incident fix.
Each off-chain workspace patches the legacy versions it uses to these copies.
They live inside the router build context so its Docker build uses the same patches.

The modern paths use upstream `rustls-webpki` 0.103.16 and
`webpki-roots`/`webpki-root-certs` 1.0.10. Keep those fixes in all four
independent off-chain lockfiles. `[patch]` entries only apply in the workspace
being built; downstream library consumers must update their own TLS dependencies.

Run the offline certificate-chain regression with:

```sh
cargo test --locked --manifest-path ncn-router/Cargo.toml --test tls_trust
```

The fixtures come from the incident site's `chains.zip`, captured on
20 October 2026. Tests fix the validation time inside their validity window
so certificate expiry cannot mask a trust regression. Remove these patches
when the upstream dependencies stop using the legacy bundles.

Rebuild and redeploy the CLIs and services after updating. Rebuild containers
with `--pull --no-cache` to refresh OS CA packages; both runtime Dockerfiles
require Debian's patched `ca-certificates` version. Hosts running native TLS
clients also need their distribution's CA update.
