//! TFS 1.4.2 XTEA (`src/xtea.cpp`): `expand_key` + block encrypt/decrypt with 64 expanded round keys.
// This differs from `crate::xtea` (32-round fixed API); wire format must match C++.

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

/// C++: `xtea::encrypt`
pub fn encrypt(data: &mut [u8], length: usize, k: &RoundKeys) {
    assert!(length <= data.len());
    for i in (0..k.len()).step_by(2) {
        let mut it = 0usize;
        while it + 8 <= length {
            let mut left = u32::from_le_bytes(data[it..it + 4].try_into().unwrap());
            let mut right = u32::from_le_bytes(data[it + 4..it + 8].try_into().unwrap());

            left = left.wrapping_add(((right << 4) ^ (right >> 5)).wrapping_add(right) ^ k[i]);
            right = right.wrapping_add(((left << 4) ^ (left >> 5)).wrapping_add(left) ^ k[i + 1]);

            data[it..it + 4].copy_from_slice(&left.to_le_bytes());
            data[it + 4..it + 8].copy_from_slice(&right.to_le_bytes());
            it += 8;
        }
    }
}

/// C++: `xtea::decrypt`
pub fn decrypt(data: &mut [u8], length: usize, k: &RoundKeys) {
    assert!(length <= data.len());
    let mut i = (k.len() as isize) - 1;
    while i > 0 {
        let mut it = 0usize;
        while it + 8 <= length {
            let mut left = u32::from_le_bytes(data[it..it + 4].try_into().unwrap());
            let mut right = u32::from_le_bytes(data[it + 4..it + 8].try_into().unwrap());

            right =
                right.wrapping_sub(((left << 4) ^ (left >> 5)).wrapping_add(left) ^ k[i as usize]);
            left = left.wrapping_sub(
                ((right << 4) ^ (right >> 5)).wrapping_add(right) ^ k[i as usize - 1],
            );

            data[it..it + 4].copy_from_slice(&left.to_le_bytes());
            data[it + 4..it + 8].copy_from_slice(&right.to_le_bytes());
            it += 8;
        }
        i -= 2;
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
}
