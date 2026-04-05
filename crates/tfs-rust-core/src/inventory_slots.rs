//! Map `player_items` / `player_storeinboxitems` rows to equipment slots (`slots_t` / `IOLoginData::loadPlayer`).
// C++ reference: `src/iologindata.cpp` (pid `CONST_SLOT_FIRST..=LAST` for inventory).

use tfs_rust_db::ItemRecord;

/// Build `[slot1..=slot10, store_inbox]` from DB rows (`pid` 1–10 = direct equipment).
pub fn build_equipment_slots(inv: &[ItemRecord], store_inbox: &[ItemRecord]) -> [Option<ItemRecord>; 11] {
    let mut slots: [Option<ItemRecord>; 11] = std::array::from_fn(|_| None);
    for r in inv {
        if (1..=10).contains(&r.pid) {
            slots[(r.pid - 1) as usize] = Some(r.clone());
        }
    }
    if let Some(r) = store_inbox
        .iter()
        .find(|r| (0..100).contains(&r.pid))
        .or_else(|| store_inbox.first())
    {
        slots[10] = Some(r.clone());
    }
    slots
}
