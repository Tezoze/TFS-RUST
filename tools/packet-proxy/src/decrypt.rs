//! XTEA decrypt for logging only — forwarded bytes are unchanged.

use tfs_rust_net::protocol_game::decrypt_xtea_game_body;
use tfs_rust_net::xtea_tfs::RoundKeys;

pub fn decrypt_game_body(body: &[u8], keys: &RoundKeys) -> Option<Vec<u8>> {
    let mut buf = body.to_vec();
    match decrypt_xtea_game_body(&mut buf, keys) {
        Ok(plain) => Some(plain.to_vec()),
        Err(_) => None,
    }
}
