const DELTA: u32 = 0x9E3779B9;

pub fn encrypt(data: &mut [u8], key: &[u32; 4]) {
    let mut i = 0;
    while i + 8 <= data.len() {
        let mut v0 = u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
        let mut v1 = u32::from_le_bytes(data[i + 4..i + 8].try_into().unwrap());
        let mut sum = 0u32;

        for _ in 0..32 {
            v0 = v0.wrapping_add(
                ((v1 << 4 ^ v1 >> 5).wrapping_add(v1))
                    ^ (sum.wrapping_add(key[(sum & 3) as usize])),
            );
            sum = sum.wrapping_add(DELTA);
            v1 = v1.wrapping_add(
                ((v0 << 4 ^ v0 >> 5).wrapping_add(v0))
                    ^ (sum.wrapping_add(key[((sum >> 11) & 3) as usize])),
            );
        }

        data[i..i + 4].copy_from_slice(&v0.to_le_bytes());
        data[i + 4..i + 8].copy_from_slice(&v1.to_le_bytes());
        i += 8;
    }
}

pub fn decrypt(data: &mut [u8], key: &[u32; 4]) {
    let mut i = 0;
    while i + 8 <= data.len() {
        let mut v0 = u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
        let mut v1 = u32::from_le_bytes(data[i + 4..i + 8].try_into().unwrap());
        let mut sum = DELTA.wrapping_mul(32);

        for _ in 0..32 {
            v1 = v1.wrapping_sub(
                ((v0 << 4 ^ v0 >> 5).wrapping_add(v0))
                    ^ (sum.wrapping_add(key[((sum >> 11) & 3) as usize])),
            );
            sum = sum.wrapping_sub(DELTA);
            v0 = v0.wrapping_sub(
                ((v1 << 4 ^ v1 >> 5).wrapping_add(v1))
                    ^ (sum.wrapping_add(key[(sum & 3) as usize])),
            );
        }

        data[i..i + 4].copy_from_slice(&v0.to_le_bytes());
        data[i + 4..i + 8].copy_from_slice(&v1.to_le_bytes());
        i += 8;
    }
}
