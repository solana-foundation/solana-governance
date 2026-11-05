use rustls_pki_types::{CertificateDer, UnixTime};
use std::{fs, io::BufReader, path::Path, time::Duration};

const VALIDATION_TIME: u64 = 1_792_540_800; // 2026-10-21, within the fixtures' validity period.

fn certificates(path: &Path) -> Vec<CertificateDer<'static>> {
    rustls_pemfile::certs(&mut BufReader::new(fs::File::open(path).unwrap()))
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn compromised_emudhra_chains_are_rejected_by_all_bundled_root_versions() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/emudhra");
    let removed_roots = certificates(&directory.join("removed-roots.pem"));
    let original_roots: Vec<_> = removed_roots
        .iter()
        .map(|cert| webpki_legacy::TrustAnchor::try_from_cert_der(cert.as_ref()).unwrap())
        .collect();
    let modern_original_roots: Vec<_> = removed_roots
        .iter()
        .map(|cert| webpki::anchor_from_trusted_cert(cert).unwrap())
        .collect();
    let modern_certificate_roots: Vec<_> = webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .map(|cert| webpki::anchor_from_trusted_cert(cert).unwrap())
        .collect();
    let roots_25: Vec<_> = webpki_roots_25::TLS_SERVER_ROOTS
        .iter()
        .map(|root| webpki_legacy::TrustAnchor {
            subject: root.subject,
            spki: root.spki,
            name_constraints: root.name_constraints,
        })
        .collect();
    let algorithms = &[
        &webpki_legacy::ECDSA_P256_SHA256,
        &webpki_legacy::ECDSA_P256_SHA384,
        &webpki_legacy::ECDSA_P384_SHA256,
        &webpki_legacy::ECDSA_P384_SHA384,
        &webpki_legacy::ED25519,
        &webpki_legacy::RSA_PKCS1_2048_8192_SHA256,
        &webpki_legacy::RSA_PKCS1_2048_8192_SHA384,
        &webpki_legacy::RSA_PKCS1_2048_8192_SHA512,
        &webpki_legacy::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
        &webpki_legacy::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
        &webpki_legacy::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
    ];

    for filename in [
        "dv-g3b-emudhra.we-have-a-problem.com.pem",
        "dv-g3b-aaa-emudhra.we-have-a-problem.com.pem",
    ] {
        let path = directory.join(filename);
        let chain = certificates(&path);
        let leaf = webpki_legacy::EndEntityCert::try_from(chain[0].as_ref()).unwrap();
        let intermediates: Vec<_> = chain[1..].iter().map(|cert| cert.as_ref()).collect();
        let verify = |roots: &[webpki_legacy::TrustAnchor<'_>]| {
            leaf.verify_for_usage(
                algorithms,
                roots,
                &intermediates,
                webpki_legacy::Time::from_seconds_since_unix_epoch(VALIDATION_TIME),
                webpki_legacy::KeyUsage::server_auth(),
                &[],
            )
        };

        // A valid chain against the removed roots rules out expiry or unsupported algorithms
        // masking a regression in the trust bundles.
        assert_eq!(verify(&original_roots), Ok(()), "{}", path.display());
        for roots in [webpki_roots_24::TLS_SERVER_ROOTS.0, roots_25.as_slice()] {
            assert_eq!(
                verify(roots),
                Err(webpki_legacy::Error::UnknownIssuer),
                "{}",
                path.display()
            );
        }

        let modern_leaf = webpki::EndEntityCert::try_from(&chain[0]).unwrap();
        let modern_error = |roots: &[rustls_pki_types::TrustAnchor<'_>]| {
            modern_leaf
                .verify_for_usage(
                    webpki::ALL_VERIFICATION_ALGS,
                    roots,
                    &chain[1..],
                    UnixTime::since_unix_epoch(Duration::from_secs(VALIDATION_TIME)),
                    webpki::KeyUsage::server_auth(),
                    None,
                    None,
                )
                .err()
        };
        assert_eq!(
            modern_error(&modern_original_roots),
            Some(webpki::Error::CertDistrusted),
            "{}",
            path.display()
        );
        for roots in [
            webpki_roots::TLS_SERVER_ROOTS,
            modern_certificate_roots.as_slice(),
        ] {
            assert!(
                matches!(
                    modern_error(roots),
                    Some(webpki::Error::CertDistrusted | webpki::Error::UnknownIssuer)
                ),
                "{}",
                path.display()
            );
        }
    }
}
