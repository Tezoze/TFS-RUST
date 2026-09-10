//! Sid/pid rebase when appending serialized items onto an existing depot table.
//! C++ reference: `IOLoginData::saveItems` runningId offset (`iologindata.cpp`).

use tfs_rust_db::items::ItemRecord;

use crate::formulas::DepotLockerStructure;

/// 772 locker-root pid (`game_world_save.rs` / `load_depot_table`): `0x10000 + town_id`.
pub const LOCKER_ROOT_PID_BASE: i32 = 0x10000;

/// Offline depot-table parent for mail / house dump.
/// 772: locker-loose (`CleanHouse` `CreateTempDepot` / `SendMails`). 1098: town chest pid.
pub fn depot_table_root_pid(structure: DepotLockerStructure, town_id: u32) -> i32 {
    match structure {
        DepotLockerStructure::ClassicDepotChest => LOCKER_ROOT_PID_BASE + town_id as i32,
        DepotLockerStructure::TfsMarketInbox => town_id as i32,
    }
}

/// Town-chest roots (`pid` 0–99) and locker roots (`0x10000 + town`) stay unshifted.
fn is_depot_table_root_pid(pid: i32) -> bool {
    (0..100).contains(&pid)
        || (LOCKER_ROOT_PID_BASE..LOCKER_ROOT_PID_BASE + 100).contains(&pid)
}

/// Shift freshly serialized rows (sids starting at 101) so they sit after `max_sid`.
///
/// Nested `pid` values are parent sids from the same tree and must move with `sid`.
/// Town-chest roots use `pid = town_id` (typically 1–99) and stay unshifted.
/// Locker-loose mail uses `pid = 0x10000 + town_id` and also stays unshifted.
pub fn apply_sid_pid_offset(records: &mut [ItemRecord], max_sid: i32) {
    let offset = max_sid.saturating_sub(100);
    for rec in records {
        if !is_depot_table_root_pid(rec.pid) && rec.pid > 99 {
            rec.pid += offset;
        }
        rec.sid += offset;
    }
}

/// Append `extra` onto `rows` after rebasing against the current max sid.
pub fn append_offset_records(rows: &mut Vec<ItemRecord>, mut extra: Vec<ItemRecord>) {
    let max_sid = rows.iter().map(|r| r.sid).max().unwrap_or(100);
    apply_sid_pid_offset(&mut extra, max_sid);
    rows.extend(extra);
}

/// `SendMails` prepends mail bytes ahead of the existing depot blob (`moveuse.cc:883-899`).
/// `load_depot_table` sorts sid descending then `internal_add_item_front`, so **lowest sid**
/// among locker-root children becomes slot 0 (UI top). New rows keep 101-based sids;
/// existing rows shift up.
pub fn prepend_offset_records(rows: &mut Vec<ItemRecord>, extra: Vec<ItemRecord>) {
    let shift = extra
        .iter()
        .map(|r| r.sid)
        .max()
        .unwrap_or(100)
        .saturating_sub(100);
    if shift > 0 {
        for rec in rows.iter_mut() {
            if !is_depot_table_root_pid(rec.pid) && rec.pid > 99 {
                rec.pid += shift;
            }
            rec.sid += shift;
        }
    }
    let mut combined = extra;
    combined.append(rows);
    *rows = combined;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::DepotLockerStructure;

    fn rec(pid: i32, sid: i32, itemtype: u16) -> ItemRecord {
        ItemRecord {
            pid,
            sid,
            itemtype,
            count: 1,
            attributes: Vec::new(),
        }
    }

    #[test]
    fn town_root_pid_stays_unshifted() {
        let mut extra = vec![rec(1, 101, 2598), rec(101, 102, 2599)];
        apply_sid_pid_offset(&mut extra, 150);
        assert_eq!(extra[0].pid, 1);
        assert_eq!(extra[0].sid, 151);
        assert_eq!(extra[1].pid, 151);
        assert_eq!(extra[1].sid, 152);
    }

    #[test]
    fn locker_root_pid_stays_unshifted() {
        let mut extra = vec![rec(LOCKER_ROOT_PID_BASE + 1, 101, 2598), rec(101, 102, 2599)];
        apply_sid_pid_offset(&mut extra, 150);
        assert_eq!(extra[0].pid, LOCKER_ROOT_PID_BASE + 1);
        assert_eq!(extra[0].sid, 151);
        assert_eq!(extra[1].pid, 151);
        assert_eq!(extra[1].sid, 152);
    }

    #[test]
    fn append_rebases_against_existing_max_sid() {
        let mut rows = vec![rec(1, 140, 2594)];
        append_offset_records(&mut rows, vec![rec(1, 101, 2598)]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].pid, 1);
        assert_eq!(rows[1].sid, 141);
    }

    #[test]
    fn prepend_keeps_new_rows_at_lowest_sids() {
        let mut rows = vec![rec(LOCKER_ROOT_PID_BASE + 1, 101, 2148), rec(101, 102, 2599)];
        prepend_offset_records(
            &mut rows,
            vec![
                rec(LOCKER_ROOT_PID_BASE + 1, 101, 2598),
                rec(101, 102, 2599),
            ],
        );
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].itemtype, 2598);
        assert_eq!(rows[0].sid, 101);
        assert_eq!(rows[1].pid, 101);
        assert_eq!(rows[1].sid, 102);
        assert_eq!(rows[2].itemtype, 2148);
        assert_eq!(rows[2].sid, 103);
        assert_eq!(rows[3].pid, 103);
        assert_eq!(rows[3].sid, 104);
    }

    #[test]
    fn locker_structure_uses_locker_root_pid() {
        assert_eq!(
            depot_table_root_pid(DepotLockerStructure::ClassicDepotChest, 1),
            LOCKER_ROOT_PID_BASE + 1
        );
        assert_eq!(
            depot_table_root_pid(DepotLockerStructure::TfsMarketInbox, 1),
            1
        );
    }
}
