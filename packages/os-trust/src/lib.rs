//! Pin a Linux process to a TLS trust store that excludes the compromised
//! eMudhra roots.
//!
//! [`install`] must run on the first line of `main`, before Sentry,
//! telemetry, or any HTTP client. `native-tls` 0.2.18 caches the
//! `openssl-probe` result the first time a connector is built and loads
//! every directory that result contains. On Amazon Linux 2023 that
//! directory still has emSign hash links, and `SSL_CERT_FILE` alone does
//! not stop the load.
//!
//! Other operating systems are left alone so desktop builds keep using
//! the OS keychain.

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::io::{self, Write};
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::sync::Once;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use sha2::{Digest, Sha256};

/// SHA-256 fingerprints of the eMudhra roots that must not be trusted.
///
/// The container image check uses the same four values. Keep the two lists
/// together when either one changes.
pub const DISTRUSTED_SHA256: [&str; 4] = [
    // emSign Root TLS CA - G3
    "7dd78d5f4f13459a83dff9abbba62edbaf6f2d102bf257fd712f4d9f2746ed8d",
    // emSign Root CS CA - G3
    "f2edaaff3d9b701fb4d14c9f285f8c0bad90fc3bcc1fa07d47f4718edaba34a5",
    // AAA Certificate Services
    "d7a7a0fb5d7e2731d771e9484ebcdef71d5f0c3e0a2948782bc83ee0ea699ef4",
    // emSign ECC Root CA - G3
    "86a1ecba089c4a8d3bbe2734c612ba341d813e043cf9e8a862cd5c57a36bbe6b",
];

/// Bundle written by [`install`]. `SSL_CERT_FILE` is pointed here.
pub const PINNED_CERT_FILE: &str = "/tmp/tachyon-os-trust/cacert.pem";

/// Empty directory written by [`install`]. `SSL_CERT_DIR` is pointed here
/// so hashed CA links under `/etc/ssl/certs` are not consulted.
pub const PINNED_CERT_DIR: &str = "/tmp/tachyon-os-trust/certs";

/// PEM bundle built from `webpki-root-certs` 1.0.11 with the distrusted
/// fingerprints removed. Used when every on-disk bundle still contains one
/// of those roots.
/// SHA-256 fingerprints of every `CERTIFICATE` block in `pem`.
pub fn certificate_fingerprints(pem: &[u8]) -> Vec<String> {
    certificate_ders(pem)
        .iter()
        .map(|der| sha256_hex(der))
        .collect()
}

pub fn vetted_root_pem() -> Vec<u8> {
    let mut pem = Vec::new();
    for cert in webpki_root_certs::TLS_SERVER_ROOT_CERTS {
        let der = cert.as_ref();
        if is_distrusted_der(der) {
            continue;
        }
        pem.extend(encode_pem(der).into_bytes());
    }
    pem
}

/// Pin this process's OpenSSL trust store.
///
/// On Linux, writes a certificate bundle and an empty certificate directory
/// under `/tmp/tachyon-os-trust`, then sets `SSL_CERT_FILE` and
/// `SSL_CERT_DIR`. A bundle that already excludes the distrusted roots is
/// kept, so extra CAs concatenated into a patched distribution bundle stay
/// in use. A bundle that still contains one of the roots is replaced by
/// [`vetted_root_pem`]. The certificate directory is always the empty
/// directory: hash links are not a second copy we can trust.
///
/// Panics if the directory or the file cannot be created. Continuing would
/// leave `native-tls` on the operating system hash directory.
///
/// On non-Linux targets this is a no-op.
pub fn install() {
    #[cfg(target_os = "linux")]
    {
        static INSTALLED: Once = Once::new();
        INSTALLED.call_once(|| {
            if let Err(err) = install_linux() {
                panic!(
                    "os-trust: failed to pin the TLS trust store: {err}"
                );
            }
        });
    }
}

#[cfg(target_os = "linux")]
fn install_linux() -> io::Result<()> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("SSL_CERT_FILE") {
        if let Ok(bytes) = fs::read(&path) {
            candidates.push(bytes);
        }
    }
    for path in LINUX_BUNDLE_FILES {
        if let Ok(bytes) = fs::read(path) {
            candidates.push(bytes);
        }
    }
    let pem = select_bundle_from_candidates(&candidates);
    if pem.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "vetted root bundle is empty",
        ));
    }

    let root = Path::new("/tmp/tachyon-os-trust");
    fs::create_dir_all(root)?;
    set_private_dir(root)?;
    let cert_dir = Path::new(PINNED_CERT_DIR);
    fs::create_dir_all(cert_dir)?;
    set_private_dir(cert_dir)?;
    empty_directory(cert_dir)?;
    write_private(Path::new(PINNED_CERT_FILE), &pem)?;

    // Safety: called once from the main thread before other threads exist
    // and before native-tls caches the OpenSSL probe. The variables name
    // paths this function just created.
    unsafe {
        std::env::set_var("SSL_CERT_FILE", PINNED_CERT_FILE);
        std::env::set_var("SSL_CERT_DIR", PINNED_CERT_DIR);
    }
    eprintln!(
        "os-trust: pinned TLS trust store file={PINNED_CERT_FILE} dir={PINNED_CERT_DIR}"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn set_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(target_os = "linux")]
fn empty_directory(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let tmp = path.with_extension("pem.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(tmp, path)
}

/// Linux bundle locations, in the same order as openssl-probe 0.2.1.
#[cfg(target_os = "linux")]
const LINUX_BUNDLE_FILES: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/ssl/ca-bundle.pem",
    "/etc/pki/tls/cacert.pem",
    "/etc/ssl/cert.pem",
    "/opt/etc/ssl/certs/ca-certificates.crt",
    "/etc/ssl/certs/cacert.pem",
];

#[cfg(any(target_os = "linux", test))]
fn select_bundle_from_candidates(candidates: &[Vec<u8>]) -> Vec<u8> {
    for candidate in candidates {
        if is_acceptable_bundle(candidate) {
            return candidate.clone();
        }
    }
    vetted_root_pem()
}

#[cfg(any(target_os = "linux", test))]
fn is_acceptable_bundle(bytes: &[u8]) -> bool {
    if bytes
        .windows(b"BEGIN TRUSTED CERTIFICATE".len())
        .any(|window| window == b"BEGIN TRUSTED CERTIFICATE")
    {
        return false;
    }
    let ders = certificate_ders(bytes);
    !ders.is_empty() && ders.iter().all(|der| !is_distrusted_der(der))
}

fn is_distrusted_der(der: &[u8]) -> bool {
    let fingerprint = sha256_hex(der);
    DISTRUSTED_SHA256.contains(&fingerprint.as_str())
}

fn certificate_ders(pem: &[u8]) -> Vec<Vec<u8>> {
    let text = String::from_utf8_lossy(pem);
    let mut rest = text.as_ref();
    let mut ders = Vec::new();
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    while let Some(start) = rest.find(BEGIN) {
        rest = &rest[start + BEGIN.len()..];
        let Some(end) = rest.find(END) else {
            break;
        };
        let encoded: String =
            rest[..end].chars().filter(|c| !c.is_whitespace()).collect();
        rest = &rest[end + END.len()..];
        if let Ok(der) = BASE64.decode(encoded) {
            ders.push(der);
        }
    }
    ders
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn encode_pem(der: &[u8]) -> String {
    let encoded = BASE64.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).expect("base64 is ascii"));
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}

#[cfg(test)]
mod tests {
    use super::*;

    fn der_contains(der: &[u8], needle: &[u8]) -> bool {
        der.windows(needle.len()).any(|window| window == needle)
    }

    const ECC_ROOT: &[u8] =
        include_bytes!("../tests/fixtures/emsign-ecc-root-g3.crt");
    const TLS_ROOT: &[u8] =
        include_bytes!("../tests/fixtures/emsign-root-tls-ca-g3.crt");
    const CS_ROOT: &[u8] =
        include_bytes!("../tests/fixtures/emsign-root-cs-ca-g3.crt");
    const AAA_ROOT: &[u8] =
        include_bytes!("../tests/fixtures/aaa-certificate-services.crt");

    #[test]
    fn vetted_bundle_excludes_the_emudhra_roots_and_keeps_public_roots() {
        let pem = vetted_root_pem();
        let ders = certificate_ders(&pem);
        assert!(
            ders.len() > 100,
            "vetted bundle has {} certificates",
            ders.len()
        );
        assert!(
            !ders.iter().any(|der| is_distrusted_der(der)),
            "vetted bundle still contains a distrusted root"
        );
        assert!(
            ders.iter().any(|der| der_contains(der, b"Amazon Root CA")),
            "vetted bundle is missing Amazon Root CA"
        );
        assert!(
            ders.iter().any(|der| der_contains(der, b"ISRG Root X1")),
            "vetted bundle is missing ISRG Root X1"
        );
    }

    #[test]
    fn fixture_roots_are_the_distrusted_fingerprints() {
        let fixtures: [&[u8]; 4] = [ECC_ROOT, TLS_ROOT, CS_ROOT, AAA_ROOT];
        let mut found = Vec::new();
        for fixture in fixtures {
            let ders = certificate_ders(fixture);
            assert_eq!(
                ders.len(),
                1,
                "fixture is not a single certificate"
            );
            let fingerprint = sha256_hex(&ders[0]);
            assert!(
                DISTRUSTED_SHA256.contains(&fingerprint.as_str()),
                "fixture fingerprint {fingerprint} is not one of the four roots"
            );
            found.push(fingerprint);
        }
        for fingerprint in DISTRUSTED_SHA256 {
            assert!(
                found.iter().any(|found| found == fingerprint),
                "no fixture produced {fingerprint}"
            );
        }
    }

    #[test]
    fn a_dirty_bundle_is_replaced_by_the_vetted_one() {
        let clean = vetted_root_pem();
        let mut dirty = clean.clone();
        dirty.extend_from_slice(ECC_ROOT);
        assert!(!is_acceptable_bundle(&dirty));
        assert!(is_acceptable_bundle(&clean));
        assert!(!is_acceptable_bundle(b""));

        let chosen =
            select_bundle_from_candidates(&[dirty.clone(), clean.clone()]);
        assert_eq!(chosen, clean);

        let fallback = select_bundle_from_candidates(&[dirty]);
        assert!(is_acceptable_bundle(&fallback));
        assert!(!fallback
            .windows(ECC_ROOT.len())
            .any(|window| window == ECC_ROOT));
    }

    #[test]
    fn trusted_certificate_blocks_are_not_acceptable() {
        let mut pem = vetted_root_pem();
        pem.extend_from_slice(b"-----BEGIN TRUSTED CERTIFICATE-----\nMIIB\n-----END TRUSTED CERTIFICATE-----\n");
        assert!(!is_acceptable_bundle(&pem));
    }
}
