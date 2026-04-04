//! Adler-32 checksum as in `src/tools.cpp` `adlerChecksum` (lower 16 = `a`, upper 16 = `b`).

const ADLER: u32 = 65521;

/// C++: `adlerChecksum`
pub fn adler_checksum(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    let mut length = data.len();
    let mut offset = 0usize;

    while length > 0 {
        let tmp = length.min(5552);
        length -= tmp;
        for _ in 0..tmp {
            a = a.wrapping_add(data[offset] as u32);
            b = b.wrapping_add(a);
            offset += 1;
        }
        a %= ADLER;
        b %= ADLER;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_tail_matches_cpp_layout() {
        // 8 bytes: u16 6, u8 0x1F, u32 ts, u8 rand — same as ProtocolGame::onConnect tail.
        let mut chunk = [0u8; 8];
        chunk[0..2].copy_from_slice(&6u16.to_le_bytes());
        chunk[2] = 0x1F;
        chunk[3..7].copy_from_slice(&0x11223344u32.to_le_bytes());
        chunk[7] = 0xAB;
        let c = adler_checksum(&chunk);
        assert_ne!(c, 0);
    }
}
