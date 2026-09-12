//! RSA helpers for the game login block.
// C++ reference: `src/rsa.cpp` — **raw** RSA (`Integer` → `CalculateInverse` → 128-byte encode), not PKCS#1 unpadding.
// The `rsa` crate’s `Pkcs1v15Encrypt` decrypt strips padding and often fails against OTClient/TFS ciphertext.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use num_bigint_dig::BigUint;
use rsa::RsaPrivateKey;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::traits::PrivateKeyParts;
use rsa::traits::PublicKeyParts;
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

/// Raw 1024-bit RSA block encrypt: `m^e mod n`. Client side of [`decrypt`].
///
/// CryptoPP `Integer` encryption is raw modular exponentiation, not PKCS#1 padding.
pub fn encrypt(block: &[u8; 128], n: &BigUint, e: &BigUint) -> Result<[u8; 128]> {
    let m = BigUint::from_bytes_be(block.as_slice());
    if m >= *n {
        return Err(TfsRustError::Protocol("RSA plaintext >= modulus".into()));
    }
    let c = m.modpow(e, n);
    integer_to_fixed_128_be(&c)
        .ok_or_else(|| TfsRustError::Protocol("RSA ciphertext > 128 bytes".into()))
}

/// Modulus and public exponent from a PKCS#1 private key (`n`, `e`).
pub fn public_parts(key: &RsaPrivateKey) -> (BigUint, BigUint) {
    let n = BigUint::from_bytes_be(&key.n().to_bytes_be());
    let e = BigUint::from_bytes_be(&key.e().to_bytes_be());
    (n, e)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::{ProtocolCaps, ProtocolVersion};

    use crate::game_first_packet::{FirstClientPacket, LoginIdentity, parse_first_client_packet};

    fn workspace_key() -> RsaPrivateKey {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../key.pem");
        let pem = std::fs::read_to_string(&path).expect("read key.pem");
        private_key_from_pkcs1_pem(&pem).expect("relaxed PEM load")
    }

    fn put_string(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    fn rsa_plain_with_creds(xtea: [u32; 4], creds: &[u8]) -> [u8; 128] {
        let mut plain = [0xFFu8; 128];
        plain[0] = 0x00;
        for (i, w) in xtea.iter().enumerate() {
            let off = 1 + i * 4;
            plain[off..off + 4].copy_from_slice(&w.to_le_bytes());
        }
        assert!(creds.len() <= 111, "test creds must fit in RSA block");
        plain[17..17 + creds.len()].copy_from_slice(creds);
        plain
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = workspace_key();
        let (n, e) = public_parts(&key);
        let mut plain = [0xFFu8; 128];
        plain[0] = 0x00;
        for (i, b) in (1u8..=16).enumerate() {
            plain[1 + i] = b;
        }
        let cipher = encrypt(&plain, &n, &e).expect("encrypt");
        assert_ne!(cipher, plain);
        let back = decrypt(&cipher, &key).expect("decrypt");
        assert_eq!(back.as_slice(), plain.as_slice());
    }

    #[test]
    fn encrypt_login_body_parses_772() {
        let key = workspace_key();
        let (n, e) = public_parts(&key);
        let xtea = [0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444];
        let mut creds = Vec::new();
        creds.extend_from_slice(&1u32.to_le_bytes());
        put_string(&mut creds, "1");
        let rsa_plain = rsa_plain_with_creds(xtea, &creds);
        let cipher = encrypt(&rsa_plain, &n, &e).expect("encrypt");

        let mut body = Vec::with_capacity(17 + 128);
        body.push(0x01);
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&772u16.to_le_bytes());
        body.extend_from_slice(&[0u8; 12]);
        body.extend_from_slice(&cipher);

        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        match parse_first_client_packet(&body, &key, &caps).expect("parse login") {
            FirstClientPacket::Login {
                xtea_key,
                identity,
                password,
                operating_system,
                ..
            } => {
                assert_eq!(xtea_key, xtea);
                assert_eq!(identity, LoginIdentity::AccountNumber(1));
                assert_eq!(password, "1");
                assert_eq!(operating_system, 1);
            }
            other => panic!("expected Login, got {other:?}"),
        }
    }

    #[test]
    fn encrypt_game_body_parses_772() {
        let key = workspace_key();
        let (n, e) = public_parts(&key);
        let xtea = [9u32, 8, 7, 6];
        let mut creds = Vec::new();
        creds.push(0);
        creds.extend_from_slice(&1u32.to_le_bytes());
        put_string(&mut creds, "Test");
        put_string(&mut creds, "1");
        let rsa_plain = rsa_plain_with_creds(xtea, &creds);
        let cipher = encrypt(&rsa_plain, &n, &e).expect("encrypt");

        let mut body = Vec::with_capacity(5 + 128);
        body.push(0x0A);
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&772u16.to_le_bytes());
        body.extend_from_slice(&cipher);

        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        match parse_first_client_packet(&body, &key, &caps).expect("parse game") {
            FirstClientPacket::Game(g) => {
                assert_eq!(g.xtea_key, xtea);
                assert_eq!(g.identity, LoginIdentity::AccountNumber(1));
                assert_eq!(g.character_name, "Test");
                assert_eq!(g.password, "1");
                assert_eq!(g.operating_system, 1);
            }
            other => panic!("expected Game, got {other:?}"),
        }
    }
}
