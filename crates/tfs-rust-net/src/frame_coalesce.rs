//! Coalesce one beat of logical packets into one XTEA TCP frame (split if oversized).
//!
//! Corpus: `communication.cc` `SendData` / `WriteToSocket` wraps every byte committed since
//! `NextToSend` in one `[u16 enc size][XTEA([u16 DataSize][packets][pad])]` frame
//! (`sending.cc` `SendAll`, `main.cc` `AdvanceGame`). Header is written in place over a
//! 4-byte (772) / 6-byte (1098 Adler) reserve; encrypt in place (`communication.cc:261-293`).
//! Pack surface: TFS `protocol.cpp` `onSendMessage` (optional Adler). Padding is zeros here;
//! decompile uses `rand_r` — the client ignores pad bytes.

use tfs_rust_common::ProtocolCaps;

use crate::adler::adler_checksum;
use crate::xtea_tfs::RoundKeys;

/// 772 `connections.hh` `OutData[16384]` — max raw opcode bytes `SendData` wraps in one frame.
const MAX_COALESCED_PAYLOAD_772: usize = 16_384;
/// 1098 `networkmessage.h` `MAX_PROTOCOL_BODY_LENGTH` (active 1098 codec, not TVP 24572).
const MAX_COALESCED_PAYLOAD_1098: usize = 65_476;

/// Max concatenated opcode bytes in one outbound XTEA frame for `caps`.
#[must_use]
pub fn max_coalesced_payload(caps: &ProtocolCaps) -> usize {
    if caps.adler_checksum {
        MAX_COALESCED_PAYLOAD_1098
    } else {
        MAX_COALESCED_PAYLOAD_772
    }
}

fn checksum_prefix_len(caps: &ProtocolCaps) -> usize {
    if caps.adler_checksum { 4 } else { 0 }
}

/// Outer `u16` + optional Adler. Cipher (inner `u16` + payload + pad) starts after this.
fn header_reserve(caps: &ProtocolCaps) -> usize {
    2 + checksum_prefix_len(caps)
}

/// Write outer length, optional Adler, inner `v`, pad, and XTEA over `buf`.
///
/// On entry `buf` is `[header zeros][payload…]` with `buf.len() == header_reserve + 2 + payload_len`.
/// On exit `buf` is a complete TCP frame ready for `write_all`.
fn finalize_xtea_frame(
    buf: &mut Vec<u8>,
    payload_len: usize,
    keys: &RoundKeys,
    caps: &ProtocolCaps,
) {
    let checksum_len = checksum_prefix_len(caps);
    let header = 2 + checksum_len;
    let plain_len = (2 + payload_len).next_multiple_of(8);
    buf.resize(header + plain_len, 0);

    let v = payload_len as u16;
    buf[header..header + 2].copy_from_slice(&v.to_le_bytes());

    crate::xtea_tfs::encrypt(&mut buf[header..header + plain_len], plain_len, keys);

    let body_len = checksum_len + plain_len;
    buf[0..2].copy_from_slice(&(body_len as u16).to_le_bytes());
    if caps.adler_checksum {
        let checksum = adler_checksum(&buf[header..header + plain_len]);
        buf[2..6].copy_from_slice(&checksum.to_le_bytes());
    }
}

/// One logical payload → one XTEA frame in `out` (login port / tests).
/// Reuses `out`'s allocation when capacity is enough.
pub fn encode_payload_frame(
    payload: &[u8],
    keys: &RoundKeys,
    caps: &ProtocolCaps,
    out: &mut Vec<u8>,
) {
    let header = header_reserve(caps);
    let plain_len = (2 + payload.len()).next_multiple_of(8);
    out.clear();
    out.reserve(header + plain_len);
    out.resize(header + 2, 0);
    out.extend_from_slice(payload);
    finalize_xtea_frame(out, payload.len(), keys, caps);
}

/// Pack packets from the front of `packets` into one XTEA frame in `out`.
///
/// Returns how many leading packets were consumed (including skipped empty ones).
/// `out` is empty when every consumed packet was empty — caller still advances by the return.
/// A single packet larger than the era max is still one frame (opcodes are not split).
#[must_use]
pub fn encode_one_coalesced_frame(
    packets: &[Vec<u8>],
    keys: &RoundKeys,
    caps: &ProtocolCaps,
    out: &mut Vec<u8>,
) -> usize {
    encode_one_coalesced_frame_with_limit(packets, keys, caps, out, max_coalesced_payload(caps))
}

fn encode_one_coalesced_frame_with_limit(
    packets: &[Vec<u8>],
    keys: &RoundKeys,
    caps: &ProtocolCaps,
    out: &mut Vec<u8>,
    max_payload: usize,
) -> usize {
    if packets.is_empty() {
        out.clear();
        return 0;
    }

    let header = header_reserve(caps);
    out.clear();
    out.resize(header + 2, 0);

    let mut payload_len = 0usize;
    let mut consumed = 0usize;
    for p in packets {
        if p.is_empty() {
            consumed += 1;
            continue;
        }
        if payload_len > 0 && payload_len.saturating_add(p.len()) > max_payload {
            break;
        }
        out.extend_from_slice(p);
        payload_len += p.len();
        consumed += 1;
        if payload_len >= max_payload {
            break;
        }
    }

    if payload_len == 0 {
        out.clear();
        return consumed;
    }

    finalize_xtea_frame(out, payload_len, keys, caps);
    consumed
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::ProtocolVersion;

    use crate::protocol_game::decrypt_xtea_game_body;
    use crate::xtea_tfs::expand_key;

    fn keys() -> RoundKeys {
        expand_key(&[1u32, 2, 3, 4])
    }

    fn decrypt_frame(frame: &[u8], keys: &RoundKeys, caps: &ProtocolCaps) -> Vec<u8> {
        let body_len = u16::from_le_bytes([frame[0], frame[1]]) as usize;
        assert_eq!(
            frame.len(),
            2 + body_len,
            "outer length must cover the frame"
        );
        let mut body = frame[2..].to_vec();
        decrypt_xtea_game_body(&mut body, keys, caps)
            .expect("decrypt coalesced frame")
            .to_vec()
    }

    fn encode_all(
        packets: &[Vec<u8>],
        keys: &RoundKeys,
        caps: &ProtocolCaps,
        max_payload: usize,
    ) -> Vec<Vec<u8>> {
        let mut rest = packets;
        let mut frames = Vec::new();
        let mut scratch = Vec::new();
        while !rest.is_empty() {
            let n =
                encode_one_coalesced_frame_with_limit(rest, keys, caps, &mut scratch, max_payload);
            assert!(n > 0, "must consume at least one packet per call");
            if !scratch.is_empty() {
                frames.push(scratch.clone());
            }
            rest = &rest[n..];
        }
        frames
    }

    #[test]
    fn max_payload_is_era_tuned_not_tvp_24572() {
        let caps_772 = ProtocolCaps::for_version(ProtocolVersion::V772);
        let caps_1098 = ProtocolCaps::for_version(ProtocolVersion::V1098);
        assert_eq!(max_coalesced_payload(&caps_772), 16_384);
        assert_eq!(max_coalesced_payload(&caps_1098), 65_476);
        assert_ne!(max_coalesced_payload(&caps_772), 24_572);
        assert_ne!(max_coalesced_payload(&caps_1098), 24_572);
    }

    #[test]
    fn n_packets_one_frame_decrypts_to_concat_772() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let packets = vec![vec![0x6Du8, 1, 2, 3], vec![0x83, 4], vec![0xB5, 0]];
        let frames = encode_all(&packets, &keys(), &caps, max_coalesced_payload(&caps));
        assert_eq!(frames.len(), 1);
        let body_len = u16::from_le_bytes([frames[0][0], frames[0][1]]) as usize;
        assert!(
            body_len.is_multiple_of(8),
            "772 body is pure XTEA blocks, no Adler"
        );
        let concat: Vec<u8> = packets.iter().flatten().copied().collect();
        assert_eq!(decrypt_frame(&frames[0], &keys(), &caps), concat);
    }

    #[test]
    fn n_packets_one_frame_decrypts_to_concat_1098() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);
        let packets = vec![vec![0x6Du8, 1, 2, 3], vec![0x83, 4], vec![0xB5, 0]];
        let frames = encode_all(&packets, &keys(), &caps, max_coalesced_payload(&caps));
        assert_eq!(frames.len(), 1);
        let body_len = u16::from_le_bytes([frames[0][0], frames[0][1]]) as usize;
        assert_eq!(body_len % 8, 4, "1098 body is Adler(4) + XTEA blocks");
        let concat: Vec<u8> = packets.iter().flatten().copied().collect();
        assert_eq!(decrypt_frame(&frames[0], &keys(), &caps), concat);
    }

    #[test]
    fn coalesced_frame_matches_single_payload_encrypt() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let packets = [vec![0x14u8, 1, 2, 3], vec![0x65, 0, 1]];
        let concat: Vec<u8> = packets.iter().flatten().copied().collect();
        let mut expected = Vec::new();
        encode_payload_frame(&concat, &keys(), &caps, &mut expected);
        let frames = encode_all(&packets, &keys(), &caps, 1024);
        assert_eq!(frames, vec![expected]);
    }

    #[test]
    fn oversized_beat_splits_into_k_decodable_frames() {
        for version in [ProtocolVersion::V772, ProtocolVersion::V1098] {
            let caps = ProtocolCaps::for_version(version);
            let packets = vec![vec![1u8; 10], vec![2u8; 10], vec![3u8; 10]];
            let frames = encode_all(&packets, &keys(), &caps, 16);
            assert_eq!(frames.len(), 3, "{version} must split at 16-byte cap");
            for (frame, packet) in frames.iter().zip(&packets) {
                assert_eq!(decrypt_frame(frame, &keys(), &caps), packet.as_slice());
            }
        }
    }

    #[test]
    fn single_packet_larger_than_max_is_still_one_frame() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let big = vec![0x64u8; 20];
        let frames = encode_all(std::slice::from_ref(&big), &keys(), &caps, 8);
        assert_eq!(frames.len(), 1);
        assert_eq!(decrypt_frame(&frames[0], &keys(), &caps), big);
    }

    #[test]
    fn reusable_scratch_does_not_reallocate_when_capacity_suffices() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let packets = vec![vec![0x6Du8, 1], vec![0x6D, 2], vec![0x6D, 3]];
        let mut scratch = Vec::with_capacity(1024);
        let cap = scratch.capacity();
        for _ in 0..8 {
            let n = encode_one_coalesced_frame(&packets, &keys(), &caps, &mut scratch);
            assert_eq!(n, packets.len());
            assert!(!scratch.is_empty());
            assert_eq!(scratch.capacity(), cap);
        }
    }

    #[test]
    fn empty_packets_are_skipped_without_a_frame() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let mut scratch = Vec::new();
        let n = encode_one_coalesced_frame(&[Vec::new(), Vec::new()], &keys(), &caps, &mut scratch);
        assert_eq!(n, 2);
        assert!(scratch.is_empty());
    }
}
