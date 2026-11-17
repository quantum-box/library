//! Proves the pinned trust store rejects the eMudhra chains.
//!
//! The host CA bundle may already be patched, so "the OS store does not
//! contain emSign" is not the assertion. The test plants the public roots
//! into a temporary hash directory and shows OpenSSL accepts the incident
//! chains from that directory, then shows `openssl_probe` 0.2.1 (patched)
//! does not return `/etc/ssl/certs` once `SSL_CERT_DIR` names another
//! directory. `native-tls` 0.2.18 loads every directory the probe returns
//! (`openssl.rs`, `TlsConnector::new`).
//!
//! `os_trust::install()` is a no-op outside Linux, so the test only runs
//! on Linux.

#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use openssl::ssl::SslFiletype;
use openssl::stack::Stack;
use openssl::x509::store::{X509Lookup, X509Store, X509StoreBuilder};
use openssl::x509::verify::X509VerifyParam;
use openssl::x509::{X509StoreContext, X509};

const EMSIGN_CHAIN: &[u8] = include_bytes!("fixtures/dv-g3b-emudhra.crt");
const AAA_CHAIN: &[u8] = include_bytes!("fixtures/dv-g3b-aaa-emudhra.crt");
const ECC_ROOT: &[u8] = include_bytes!("fixtures/emsign-ecc-root-g3.crt");
const AAA_ROOT: &[u8] =
    include_bytes!("fixtures/aaa-certificate-services.crt");

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 2026-11-01T00:00:00Z. The leaf fixtures are valid from 2026-10-20 to
/// 2027-05-03, so verification is pinned to a time inside that window.
/// Otherwise the positive controls would start failing once the fixtures
/// expire, even if the trust store behaves correctly.
const FIXTURE_VERIFY_TIME: i64 = 1_793_491_200;

#[test]
fn emsign_roots_are_not_trusted_after_install() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _restore = EnvRestore::capture();

    let root = std::env::temp_dir()
        .join(format!("os-trust-emsign-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("temp dir");

    let clean = root.join("clean.pem");
    fs::write(&clean, os_trust::vetted_root_pem()).expect("clean pem");
    let empty = root.join("empty");
    fs::create_dir_all(&empty).expect("empty dir");

    set_env("SSL_CERT_FILE", clean.to_str().expect("utf8 path"));
    set_env("SSL_CERT_DIR", empty.to_str().expect("utf8 path"));
    let probed = openssl_probe::probe();
    assert_eq!(
        probed.cert_dir,
        vec![empty.clone()],
        "SSL_CERT_DIR must replace /etc/ssl/certs, got {:?}",
        probed.cert_dir
    );
    assert!(probed.cert_file.as_deref() == Some(clean.as_path()));

    let store =
        store_loaded_like_native_tls(Some(&clean), &[empty.as_path()]);
    assert!(
        trusted_certificate_count(&store) > 50,
        "clean bundle did not load into the store"
    );
    assert!(
        !chain_verifies(&store, EMSIGN_CHAIN),
        "clean bundle plus an empty dir still trusted the emSign chain"
    );
    assert!(
        !chain_verifies(&store, AAA_CHAIN),
        "clean bundle plus an empty dir still trusted the AAA chain"
    );

    let planted = root.join("planted-ecc");
    fs::create_dir_all(&planted).expect("planted dir");
    let ecc_hash = plant_root(&planted, ECC_ROOT);
    assert_eq!(
        format!("{ecc_hash:08x}"),
        "14bc7599",
        "emSign ECC subject hash drifted from the Amazon Linux link name"
    );
    set_env("SSL_CERT_DIR", planted.to_str().expect("utf8 path"));
    let probed = openssl_probe::probe();
    assert_eq!(probed.cert_dir, vec![planted.clone()]);
    let trusting =
        store_loaded_like_native_tls(Some(&clean), &[planted.as_path()]);
    assert!(
        chain_verifies(&trusting, EMSIGN_CHAIN),
        "a hash directory containing emSign ECC Root CA - G3 did not make the chain verify"
    );

    let planted_aaa = root.join("planted-aaa");
    fs::create_dir_all(&planted_aaa).expect("planted aaa dir");
    let aaa_hash = plant_root(&planted_aaa, AAA_ROOT);
    assert_eq!(format!("{aaa_hash:08x}"), "ee64a828");
    let trusting_aaa = store_loaded_like_native_tls(
        Some(&clean),
        &[planted_aaa.as_path()],
    );
    assert!(
        chain_verifies(&trusting_aaa, AAA_CHAIN),
        "a hash directory containing AAA Certificate Services did not make the chain verify"
    );

    // rustls-native-certs 0.8 reads both SSL_CERT_FILE and SSL_CERT_DIR
    // through openssl-probe 0.1, which already treats the directory as a
    // replacement. The planted directory must show up, and the empty one
    // must not.
    set_env("SSL_CERT_DIR", planted.to_str().expect("utf8 path"));
    let loaded = rustls_native_certs::load_native_certs();
    assert!(
        loaded.errors.is_empty(),
        "rustls-native-certs errors: {:?}",
        loaded.errors
    );
    assert!(
        fingerprints(&loaded.certs).iter().any(|fingerprint| {
            fingerprint
                == "86a1ecba089c4a8d3bbe2734c612ba341d813e043cf9e8a862cd5c57a36bbe6b"
        }),
        "rustls-native-certs ignored the certificate directory"
    );

    set_env("SSL_CERT_DIR", empty.to_str().expect("utf8 path"));
    let loaded = rustls_native_certs::load_native_certs();
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    assert_no_distrusted(&fingerprints(&loaded.certs));

    unset_env("SSL_CERT_DIR");
    let probed = openssl_probe::probe();
    assert!(
        probed.cert_dir.iter().any(|path| path == Path::new("/etc/ssl/certs")),
        "an unset SSL_CERT_DIR should still fall back to the system directory, got {:?}",
        probed.cert_dir
    );

    let mut dirty = os_trust::vetted_root_pem();
    dirty.extend_from_slice(ECC_ROOT);
    let dirty_path = root.join("dirty.pem");
    fs::write(&dirty_path, &dirty).expect("dirty pem");
    set_env("SSL_CERT_FILE", dirty_path.to_str().expect("utf8 path"));
    unset_env("SSL_CERT_DIR");

    // First native-tls connector in this process. It caches the probe, so
    // this has to run after install() and after the cases above.
    os_trust::install();
    let probed = openssl_probe::probe();
    assert_eq!(
        probed.cert_file.as_deref(),
        Some(Path::new(os_trust::PINNED_CERT_FILE))
    );
    assert_eq!(
        probed.cert_dir,
        vec![PathBuf::from(os_trust::PINNED_CERT_DIR)]
    );
    let pinned =
        fs::read(os_trust::PINNED_CERT_FILE).expect("pinned bundle");
    assert_no_distrusted(&os_trust::certificate_fingerprints(&pinned));
    let pinned_ders = X509::stack_from_pem(&pinned).expect("pinned pem");
    assert!(
        pinned_ders.iter().any(|cert| {
            cert.to_der()
                .expect("der")
                .windows(b"Amazon Root CA".len())
                .any(|window| window == b"Amazon Root CA")
        }),
        "pinned bundle is missing Amazon Root CA"
    );
    assert_ne!(
        pinned, dirty,
        "install kept the bundle that contains emSign"
    );

    let pinned_store = store_loaded_like_native_tls(
        Some(Path::new(os_trust::PINNED_CERT_FILE)),
        &[Path::new(os_trust::PINNED_CERT_DIR)],
    );
    assert!(trusted_certificate_count(&pinned_store) > 50);
    assert!(!chain_verifies(&pinned_store, EMSIGN_CHAIN));
    assert!(!chain_verifies(&pinned_store, AAA_CHAIN));

    let loaded = rustls_native_certs::load_native_certs();
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    assert!(loaded.certs.len() > 50);
    assert_no_distrusted(&fingerprints(&loaded.certs));

    native_tls::TlsConnector::builder()
        .build()
        .expect("native-tls connector should build from the pinned paths");

    let _ = fs::remove_dir_all(&root);
}

fn plant_root(dir: &Path, pem: &[u8]) -> u32 {
    let cert = X509::from_pem(pem).expect("root pem");
    let hash = cert.subject_name_hash();
    fs::write(dir.join(format!("{hash:08x}.0")), pem).expect("hash link");
    hash
}

fn store_loaded_like_native_tls(
    file: Option<&Path>,
    dirs: &[&Path],
) -> X509Store {
    let mut builder = X509StoreBuilder::new().expect("store");
    if let Some(file) = file {
        builder
            .add_lookup(X509Lookup::file())
            .expect("file lookup")
            .load_cert_file(file, SslFiletype::PEM)
            .unwrap_or_else(|err| panic!("load {}: {err}", file.display()));
    }
    if !dirs.is_empty() {
        let lookup = builder
            .add_lookup(X509Lookup::hash_dir())
            .expect("hash lookup");
        for dir in dirs {
            let name = dir.to_str().expect("utf8 dir");
            lookup
                .add_dir(name, SslFiletype::PEM)
                .unwrap_or_else(|err| panic!("add dir {name}: {err}"));
        }
    }
    let mut param = X509VerifyParam::new().expect("verify param");
    param.set_time(FIXTURE_VERIFY_TIME as _);
    builder.set_param(&param).expect("set verify param");
    builder.build()
}

fn chain_verifies(store: &X509Store, chain_pem: &[u8]) -> bool {
    let certs = X509::stack_from_pem(chain_pem).expect("chain pem");
    assert!(certs.len() >= 2, "chain is missing intermediates");
    let mut untrusted = Stack::new().expect("stack");
    for cert in certs.iter().skip(1) {
        untrusted.push(cert.clone()).expect("push intermediate");
    }
    let mut context = X509StoreContext::new().expect("store context");
    context
        .init(store, &certs[0], &untrusted, |context| {
            context.verify_cert()
        })
        .expect("verify")
}

fn trusted_certificate_count(store: &X509Store) -> usize {
    store.all_certificates().len()
}

fn fingerprints(certs: &[impl AsRef<[u8]>]) -> Vec<String> {
    certs.iter().map(|cert| sha256_hex(cert.as_ref())).collect()
}

fn assert_no_distrusted(fingerprints: &[String]) {
    for fingerprint in os_trust::DISTRUSTED_SHA256 {
        assert!(
            !fingerprints.iter().any(|found| found == fingerprint),
            "trust store still contains {fingerprint}"
        );
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn set_env(key: &str, value: &str) {
    // Safety: this test holds ENV_LOCK and does not spawn threads that read
    // SSL_CERT_FILE or SSL_CERT_DIR.
    unsafe {
        std::env::set_var(key, value);
    }
}

fn unset_env(key: &str) {
    // Safety: same as set_env. The test restores the original values on drop.
    unsafe {
        std::env::remove_var(key);
    }
}

struct EnvRestore {
    file: Option<std::ffi::OsString>,
    dir: Option<std::ffi::OsString>,
}

impl EnvRestore {
    fn capture() -> Self {
        Self {
            file: std::env::var_os("SSL_CERT_FILE"),
            dir: std::env::var_os("SSL_CERT_DIR"),
        }
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        restore("SSL_CERT_FILE", &self.file);
        restore("SSL_CERT_DIR", &self.dir);
    }
}

fn restore(key: &str, value: &Option<std::ffi::OsString>) {
    // Safety: test teardown on the test thread. Restores the values captured
    // before the test mutated SSL_CERT_FILE and SSL_CERT_DIR.
    unsafe {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
}
