//! Deterministic byte decoding for legacy NPC scripts.
//!
//! Try UTF-8 first; on failure decode as ISO-8859-1 (latin-1).

/// Decode NPC/NDB source bytes to a UTF-8 [`String`].
pub fn decode_npc_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_passthrough() {
        assert_eq!(decode_npc_bytes(b"hello"), "hello");
    }

    #[test]
    fn latin1_umlaut() {
        // "für" in ISO-8859-1
        let bytes = b"f\xfcr";
        assert_eq!(decode_npc_bytes(bytes), "für");
    }
}
