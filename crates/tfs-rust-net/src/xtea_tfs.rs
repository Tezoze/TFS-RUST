//! XTEA with 64 expanded round keys (`src/xtea.cpp` formulas).
//!
//! Each 8-byte block is encrypted for all 32 rounds before the next block is loaded
//! (`communication.cc` `WriteToSocket` → `SymmetricKey.encrypt`). The round function matches
//! TFS `xtea::encrypt`; only the loop order differs, and the ciphertext is the same.
//! This differs from `crate::xtea` (32-round fixed API).

pub type Key = [u32; 4];
pub type RoundKeys = [u32; 64];

const DELTA: u32 = 0x9E37_79B9;

/// C++: `xtea::expand_key`
pub fn expand_key(k: &Key) -> RoundKeys {
    let mut expanded = [0u32; 64];
    let mut sum = 0u32;
    let mut next_sum = sum.wrapping_add(DELTA);
    let mut i = 0usize;
    while i < expanded.len() {
        expanded[i] = sum.wrapping_add(k[(sum & 3) as usize]);
        expanded[i + 1] = next_sum.wrapping_add(k[((next_sum >> 11) & 3) as usize]);
        sum = next_sum;
        next_sum = next_sum.wrapping_add(DELTA);
        i += 2;
    }
    expanded
}

#[inline]
fn read_u32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

#[inline]
fn write_u32(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// One block, all rounds, then the next (`communication.cc` `SymmetricKey.encrypt`).
pub fn encrypt(data: &mut [u8], length: usize, k: &RoundKeys) {
    assert!(length <= data.len());
    let mut it = 0usize;
    while it + 8 <= length {
        let mut left = read_u32(data, it);
        let mut right = read_u32(data, it + 4);
        let mut round = 0usize;
        while round < k.len() {
            left = left.wrapping_add(((right << 4) ^ (right >> 5)).wrapping_add(right) ^ k[round]);
            right =
                right.wrapping_add(((left << 4) ^ (left >> 5)).wrapping_add(left) ^ k[round + 1]);
            round += 2;
        }
        write_u32(data, it, left);
        write_u32(data, it + 4, right);
        it += 8;
    }
}

/// Inverse of [`encrypt`]: all rounds of one block before the next.
pub fn decrypt(data: &mut [u8], length: usize, k: &RoundKeys) {
    assert!(length <= data.len());
    let mut it = 0usize;
    while it + 8 <= length {
        let mut left = read_u32(data, it);
        let mut right = read_u32(data, it + 4);
        let mut round = k.len() as isize - 1;
        while round > 0 {
            right = right
                .wrapping_sub(((left << 4) ^ (left >> 5)).wrapping_add(left) ^ k[round as usize]);
            left = left.wrapping_sub(
                ((right << 4) ^ (right >> 5)).wrapping_add(right) ^ k[round as usize - 1],
            );
            round -= 2;
        }
        write_u32(data, it, left);
        write_u32(data, it + 4, right);
        it += 8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty_len() {
        let k = expand_key(&[1u32, 2, 3, 4]);
        let mut buf = [0u8; 8];
        buf[0..4].copy_from_slice(&0x01020304u32.to_le_bytes());
        buf[4..8].copy_from_slice(&0x05060708u32.to_le_bytes());
        let orig = buf;
        encrypt(&mut buf, 8, &k);
        assert_ne!(buf, orig);
        decrypt(&mut buf, 8, &k);
        assert_eq!(buf, orig);
    }

    /// Round-major loop from TFS `xtea.cpp` — ciphertext oracle for the block-major form.
    fn encrypt_round_major(data: &mut [u8], length: usize, k: &RoundKeys) {
        for i in (0..k.len()).step_by(2) {
            let mut it = 0usize;
            while it + 8 <= length {
                let mut left = read_u32(data, it);
                let mut right = read_u32(data, it + 4);
                left = left.wrapping_add(((right << 4) ^ (right >> 5)).wrapping_add(right) ^ k[i]);
                right =
                    right.wrapping_add(((left << 4) ^ (left >> 5)).wrapping_add(left) ^ k[i + 1]);
                write_u32(data, it, left);
                write_u32(data, it + 4, right);
                it += 8;
            }
        }
    }

    #[test]
    fn block_major_matches_round_major_oracle() {
        let k = expand_key(&[0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444]);
        let mut plain = [0u8; 24];
        for (i, b) in plain.iter_mut().enumerate() {
            *b = i as u8;
        }
        plain[20] = 0xAB;
        let mut round_major = plain;
        let mut block_major = plain;
        encrypt_round_major(&mut round_major, 24, &k);
        encrypt(&mut block_major, 24, &k);
        assert_eq!(block_major, round_major);
        decrypt(&mut block_major, 24, &k);
        assert_eq!(block_major, plain);
    }
}
