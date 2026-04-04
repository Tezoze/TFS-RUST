use proptest::prelude::*;
use tfs_rust_common::{PropStream, PropWriteStream};

proptest! {
    #[test]
    fn prop_stream_round_trip(
        u8_val in any::<u8>(),
        u16_val in any::<u16>(),
        u32_val in any::<u32>(),
        u64_val in any::<u64>(),
        str_val in ".*" // Generate random strings
    ) {
        let mut writer = PropWriteStream::new();
        writer.write_u8(u8_val);
        writer.write_u16(u16_val);
        writer.write_u32(u32_val);
        writer.write_u64(u64_val);
        writer.write_string(&str_val);

        let bytes = writer.finish();

        let mut reader = PropStream::new(&bytes);
        prop_assert_eq!(reader.read_u8().unwrap(), u8_val);
        prop_assert_eq!(reader.read_u16().unwrap(), u16_val);
        prop_assert_eq!(reader.read_u32().unwrap(), u32_val);
        prop_assert_eq!(reader.read_u64().unwrap(), u64_val);
        prop_assert_eq!(reader.read_string().unwrap(), str_val);
    }
}
