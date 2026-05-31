//! `decrypt_xtea_game_body` roundtrip with `encrypt_xtea_game_frame` (mirrors client → server ciphertext).

use tfs_rust_net::protocol_game::{decrypt_xtea_game_body, encrypt_xtea_game_frame};
use tfs_rust_net::xtea_tfs;
use tfs_rust_net::{ProtocolCaps, ProtocolVersion};

#[test]
fn decrypt_extracts_opcode_after_encrypt() {
    let keys = xtea_tfs::expand_key(&[1u32, 2, 3, 4]);
    let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);
    let frame = encrypt_xtea_game_frame(&[0x65, 0, 0, 0, 0, 0], &keys, &caps);
    let body_len = u16::from_le_bytes([frame[0], frame[1]]) as usize;
    let mut body = frame[2..2 + body_len].to_vec();
    let plain = decrypt_xtea_game_body(&mut body, &keys, &caps).expect("decrypt");
    assert_eq!(plain[0], 0x65);
}

/// 772 transport: no Adler checksum (`gameserver/src/protocol.cpp` `XTEA_decrypt`,
/// `networkmessage.h` `INITIAL_BUFFER_POSITION = 4`). Round-trips through pure XTEA blocks.
#[test]
fn decrypt_extracts_opcode_after_encrypt_772() {
    let keys = xtea_tfs::expand_key(&[9u32, 8, 7, 6]);
    let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
    let frame = encrypt_xtea_game_frame(&[0x14, 0xAA, 0xBB], &keys, &caps);
    let body_len = u16::from_le_bytes([frame[0], frame[1]]) as usize;
    let mut body = frame[2..2 + body_len].to_vec();
    let plain = decrypt_xtea_game_body(&mut body, &keys, &caps).expect("decrypt");
    assert_eq!(plain, &[0x14, 0xAA, 0xBB]);
}

/// Cross-profile guard: a 772 frame (no checksum) must not be mis-read as a 1098 frame. With a
/// checksum expected but absent, decode either fails the Adler check or yields a different payload —
/// never silently succeeds with the original bytes.
#[test]
fn caps_mismatch_does_not_silently_succeed() {
    let keys = xtea_tfs::expand_key(&[1u32, 2, 3, 4]);
    let caps_772 = ProtocolCaps::for_version(ProtocolVersion::V772);
    let caps_1098 = ProtocolCaps::for_version(ProtocolVersion::V1098);
    let frame = encrypt_xtea_game_frame(&[0x65, 0, 0, 0, 0, 0], &keys, &caps_772);
    let body_len = u16::from_le_bytes([frame[0], frame[1]]) as usize;
    let mut body = frame[2..2 + body_len].to_vec();
    let mismatched = decrypt_xtea_game_body(&mut body, &keys, &caps_1098);
    assert!(
        !mismatched
            .map(|p| p == [0x65, 0, 0, 0, 0, 0])
            .unwrap_or(false),
        "1098 decode of a 772 frame must not reproduce the original payload"
    );
}
