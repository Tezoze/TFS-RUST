use proptest::prelude::*;
use tfs_rust_common::Position;
use tfs_rust_net::NetworkMessage;

proptest! {
    #[test]
    fn message_round_trip(
        u8_val in any::<u8>(),
        u16_val in any::<u16>(),
        u32_val in any::<u32>(),
        u64_val in any::<u64>(),
        str_val in ".*",
        x in any::<u16>(),
        y in any::<u16>(),
        z in any::<u8>(),
    ) {
        let mut msg = NetworkMessage::new();
        msg.write_u8(u8_val);
        msg.write_u16(u16_val);
        msg.write_u32(u32_val);
        msg.write_u64(u64_val);
        msg.write_string(&str_val);
        let pos = Position::new(x, y, z);
        msg.write_position(&pos);

        prop_assert_eq!(msg.read_u8().unwrap(), u8_val);
        prop_assert_eq!(msg.read_u16().unwrap(), u16_val);
        prop_assert_eq!(msg.read_u32().unwrap(), u32_val);
        prop_assert_eq!(msg.read_u64().unwrap(), u64_val);
        prop_assert_eq!(msg.read_string().unwrap(), str_val);
        prop_assert_eq!(msg.read_position().unwrap(), pos);
    }
}
