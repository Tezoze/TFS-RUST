//! RSA helpers for the game login block.
// C++ reference: `src/rsa.cpp` — **raw** RSA (`Integer` → `CalculateInverse` → 128-byte encode), not PKCS#1 unpadding.
// The `rsa` crate’s `Pkcs1v15Encrypt` decrypt strips padding and often fails against OTClient/TFS ciphertext.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use num_bigint_dig::BigUint;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::traits::PrivateKeyParts;
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use tfs_rust_common::error::{Result, TfsRustError};

/// Raw 1024-bit RSA block decrypt, matching `RSA::decrypt` in `src/rsa.cpp`.
///
/// CryptoPP `CalculateInverse` is raw modular exponentiation: `m^d mod n`, no PKCS#1 unpadding.
/// We replicate this exactly using `num_bigint_dig` modpow.
pub fn decrypt(block: &[u8; 128], private_key: &RsaPrivateKey) -> Result<Vec<u8>> {
    let n_be = private_key.n().to_bytes_be();
    let d_be = private_key.d().to_bytes_be();
    let n = BigUint::from_bytes_be(&n_be);
    let d = BigUint::from_bytes_be(&d_be);

    // OTClient sends the ciphertext as a big-endian 128-byte integer (CryptoPP Integer convention).
    let c = BigUint::from_bytes_be(block.as_slice());

    if c >= n {
        return Err(TfsRustError::Protocol("RSA ciphertext >= modulus".into()));
    }

    // Raw modpow: m = c^d mod n  (same as CryptoPP CalculateInverse)
    let m = c.modpow(&d, &n);

    let out = integer_to_fixed_128_be(&m)
        .ok_or_else(|| TfsRustError::Protocol("RSA plaintext > 128 bytes".into()))?;

    if out[0] != 0 {
        return Err(TfsRustError::Protocol(
            "RSA plaintext does not start with 0x00 (wrong key?)".into(),
        ));
    }

    Ok(out.to_vec())
}

/// `Integer::Encode` to 128 bytes: fixed-width big-endian with leading zero bytes (`src/rsa.cpp`).
fn integer_to_fixed_128_be(m: &BigUint) -> Option<[u8; 128]> {
    let bytes = m.to_bytes_be();
    if bytes.len() > 128 {
        return None;
    }
    let mut out = [0u8; 128];
    out[128 - bytes.len()..].copy_from_slice(&bytes);
    Some(out)
}

/// PKCS#1 `BEGIN RSA PRIVATE KEY` PEM (same as TFS `key.pem`).
pub fn private_key_from_pkcs1_pem(pem: &str) -> Result<RsaPrivateKey> {
    match RsaPrivateKey::from_pkcs1_pem(pem) {
        Ok(k) => Ok(k),
        Err(_) => private_key_from_pkcs1_pem_relaxed(pem),
    }
}

fn private_key_from_pkcs1_pem_relaxed(pem: &str) -> Result<RsaPrivateKey> {
    let mut b64 = String::new();
    let mut in_body = false;
    for line in pem.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("-----BEGIN RSA PRIVATE KEY") {
            in_body = true;
            continue;
        }
        if t.starts_with("-----END RSA PRIVATE KEY") {
            break;
        }
        if in_body {
            b64.push_str(t);
        }
    }
    let der = STANDARD
        .decode(b64.as_bytes())
        .map_err(|_| TfsRustError::Protocol("PEM base64 decode failed".into()))?;
    RsaPrivateKey::from_pkcs1_der(&der)
        .map_err(|_| TfsRustError::Protocol("RSA PKCS#1 DER parse failed".into()))
}
