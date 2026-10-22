Certificate chains served by the eMudhra test hosts
====================================================

The two test chains are retained from https://emudhra.we-have-a-problem.com/chains.zip,
captured on 20 October 2026. Each is in PEM format, leaf first.
Both use the compromised EM DV TLS CA - G3B key, with different trust paths:

  dv-g3b-emudhra.we-have-a-problem.com.pem
      leaf, impacted intermediate, emSign Root TLS CA - G3 (cross-signed
      by emSign ECC Root CA - G3)

  dv-g3b-aaa-emudhra.we-have-a-problem.com.pem
      leaf, impacted intermediate cross-signed by AAA Certificate Services

These cover both root removals in the vendored bundles. The other incident
chains exercise additional compromised keys, whose blocklist coverage belongs
to upstream webpki.

removed-roots.pem contains the two removed root certificates, copied from the
published webpki-roots 0.25.4 bundle. The regression uses them as a positive
control: both chains must validate with the legacy verifier against those
roots, so expiry or unsupported algorithms cannot mask a trust regression.

Offline check
-------------

To see whether a trust store still accepts a chain, verify the leaf
against that store using the rest of the file as untrusted intermediates.
For the OpenSSL default store:

  openssl verify -show_chain -untrusted FILE.pem FILE.pem

For a specific bundle:

  openssl verify -show_chain -CAfile /path/to/bundle.pem -untrusted FILE.pem FILE.pem

"OK" means the store still trusts the compromised intermediate.
An error naming the issuer certificate means trust has been removed.
Note that openssl verify does not check revocation unless asked to.
