use proptest::prelude::*;
use tfs_rust_net::NetworkMessage;

proptest! {
    #[test]
    fn compression_round_trip(
        data in prop::collection::vec(any::<u8>(), 0..1024)
    ) {
        let mut msg = NetworkMessage::new();
        for &b in &data {
            msg.write_u8(b);
        }

        // Compress
        msg.compress().unwrap();

        // Check it decompresses perfectly
        msg.decompress().unwrap();

        for b in data {
            prop_assert_eq!(msg.read_u8().unwrap(), b);
        }
    }
}
