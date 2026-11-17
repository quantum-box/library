# openssl-probe 0.2.1 (patched)

Vendored from <https://github.com/rustls/openssl-probe> tag `0.2.1`
(`MIT OR Apache-2.0`). License texts are `LICENSE-MIT` and `LICENSE-APACHE`.

The only behavioral change is in `probe`: when `SSL_CERT_DIR` names an
existing directory, system certificate directories are not appended.
`openssl-probe` 0.1 and OpenSSL itself already treat that variable as a
replacement. Stock 0.2.1 appends every existing system directory anyway.

`native-tls` 0.2.18 loads every directory `probe` returns and caches the
result for the process. The append keeps `/etc/ssl/certs` trusted, including
the emSign hash links shipped on Amazon Linux 2023. See ADR-0170.
