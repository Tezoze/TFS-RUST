//! `decrypt_xtea_game_body` roundtrip with `encrypt_xtea_game_frame` (mirrors client → server ciphertext).

use tfs_rust_net::protocol_game::{decrypt_xtea_game_body, encrypt_xtea_game_frame};
use tfs_rust_net::xtea_tfs;

#[test]
fn decrypt_extracts_opcode_after_encrypt() {
    let keys = xtea_tfs::expand_key(&[1u32, 2, 3, 4]);
    let frame = encrypt_xtea_game_frame(&[0x65, 0, 0, 0, 0, 0], &keys);
    let body_len = u16::from_le_bytes([frame[0], frame[1]]) as usize;
    let mut body = frame[2..2 + body_len].to_vec();
    let plain = decrypt_xtea_game_body(&mut body, &keys).expect("decrypt");
    assert_eq!(plain[0], 0x65);
}
