//! Sid/pid rebase when appending serialized items onto an existing depot table.
//! C++ reference: `IOLoginData::saveItems` runningId offset (`iologindata.cpp`).

use tfs_rust_db::items::ItemRecord;

/// 772 locker-root pid (`game_world_save.rs` / `load_depot_table`): `0x10000 + town_id`.
pub const LOCKER_ROOT_PID_BASE: i32 = 0x10000;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
