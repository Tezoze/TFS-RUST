use rsa::{Pkcs1v15Encrypt, RsaPrivateKey};
use tfs_rust_common::error::{Result, TfsRustError};

pub fn decrypt(block: &[u8; 128], private_key: &RsaPrivateKey) -> Result<Vec<u8>> {
    private_key
        .decrypt(Pkcs1v15Encrypt, block)
        .map_err(|_| TfsRustError::Protocol("RSA decryption failed".to_string()))
}
