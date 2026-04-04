//! Property 10: condition merge idempotence for `Generic` ticks.
// C++ reference: `ConditionGeneric::addCondition`.

use proptest::prelude::*;
use tfs_rust_common::enums::ConditionType;
use tfs_rust_core::{add_condition_merge, ActiveCondition, ConditionData};

proptest! {
    #[test]
    fn merge_generic_idempotent(
        ticks in 1i32..10_000,
        sub_id in 0u32..16u32,
    ) {
        let incoming = ActiveCondition {
            id: 1,
            sub_id,
            ctype: ConditionType::Drunk,
            data: ConditionData::Generic { ticks },
        };
        let mut list = Vec::new();
        add_condition_merge(&mut list, incoming.clone());
        let once = list.clone();
        add_condition_merge(&mut list, incoming.clone());
        add_condition_merge(&mut list, incoming);
        prop_assert_eq!(once, list);
    }
}
