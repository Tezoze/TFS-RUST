use proptest::prelude::*;
use tfs_rust_net::xtea;

proptest! {
    #[test]
    fn xtea_round_trip(
        data in prop::collection::vec(any::<u8>(), 0..=256).prop_map(|mut v| {
            v.resize((v.len() + 7) / 8 * 8, 0);
            v
        }),
        key in any::<[u32; 4]>()
    ) {
        let mut encrypted = data.clone();
        xtea::encrypt(&mut encrypted, &key);
        xtea::decrypt(&mut encrypted, &key);
        prop_assert_eq!(encrypted, data);
    }
}
