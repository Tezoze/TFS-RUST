//! Map census for client ids that two live server rows share.
//!
//! A pair collapses only when the dropped server id is unused on the map, or the
//! two rows are the same item (OTB flags; `editorsuffix` ignored). Both sides
//! used and different blocks the catalog rebuild.

use std::collections::HashMap;
/// One shared client id and the two live server ids, in OTB order.
#[derive(Debug, Clone, Copy)]
pub struct CollisionPair {
    pub client_id: u16,
    pub keep_preferred: u16,
    pub other: u16,
    /// Same OTB flags and no XML behavior difference (`editorsuffix` ignored).
    pub identical: bool,
}

/// Census outcome for one shared client id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Survivor {
    /// Use this server row's flags and XML. The other id still remaps to `client_id`.
    Keep(u16),
    /// Both ids are on the map and the rows differ. Do not pick one.
    Blocked {
        client_id: u16,
        left: u16,
        left_count: u32,
        right: u16,
        right_count: u32,
    },
}

/// The six shared client ids. Preferred side is the row scripts or the bare type need.
/// Map-majority lock. The rare row is the one that is dropped.
///
/// 425 ← blocking disguise 4331 (169 tiles). 2474 ← plain coffin 1742 (127).
/// 2772 ← bare lever 1945. 2773 ← bare lever 1946 (decay is instance state).
/// 3617 ← plain tree 2703 (2214). 4240 ← decaying corpse 3058 (17).
pub const LOCKED_SURVIVORS: &[(u16, u16)] = &[
    (425, 4331),
    (2474, 1742),
    (2772, 1945),
    (2773, 1946),
    (3617, 2703),
    (4240, 3058),
];

pub const COLLISION_PAIRS: &[CollisionPair] = &[
    CollisionPair {
        client_id: 425,
        keep_preferred: 422,
        other: 4331,
        identical: false,
    },
    CollisionPair {
        client_id: 2474,
        keep_preferred: 1742,
        other: 4347,
        identical: false,
    },
    CollisionPair {
        client_id: 2772,
        keep_preferred: 1945,
        other: 4383,
        identical: true,
    },
    CollisionPair {
        client_id: 2773,
        keep_preferred: 1946,
        other: 4384,
        identical: false,
    },
    CollisionPair {
        client_id: 3617,
        keep_preferred: 2703,
        other: 4390,
        identical: false,
    },
    CollisionPair {
        client_id: 4240,
        keep_preferred: 3058,
        other: 4367,
        identical: false,
    },
];

pub fn decide_survivors(counts: &HashMap<u16, u32>) -> Vec<Survivor> {
    COLLISION_PAIRS
        .iter()
        .map(|pair| decide_one(pair, counts))
        .collect()
}

fn decide_one(pair: &CollisionPair, counts: &HashMap<u16, u32>) -> Survivor {
    let left_count = counts.get(&pair.keep_preferred).copied().unwrap_or(0);
    let right_count = counts.get(&pair.other).copied().unwrap_or(0);
    if left_count > 0 && right_count > 0 && !pair.identical {
        return Survivor::Blocked {
            client_id: pair.client_id,
            left: pair.keep_preferred,
            left_count,
            right: pair.other,
            right_count,
        };
    }
    if left_count == 0 && right_count > 0 {
        Survivor::Keep(pair.other)
    } else {
        Survivor::Keep(pair.keep_preferred)
    }
}

pub fn blocked(decisions: &[Survivor]) -> Vec<&Survivor> {
    decisions
        .iter()
        .filter(|d| matches!(d, Survivor::Blocked { .. }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Counts from `forgotten.otbm` on 2026-09-26 (all tiles / house tiles in the tally).
    fn forgotten_counts() -> HashMap<u16, u32> {
        HashMap::from([
            (422, 8),
            (4331, 169),
            (1742, 127),
            (4347, 1),
            (1945, 267),
            (4383, 0),
            (1946, 20),
            (4384, 0),
            (2703, 2214),
            (4390, 1),
            (3058, 17),
            (4367, 1),
        ])
    }

    #[test]
    fn forgotten_blocks_four_pairs_and_keeps_levers() {
        let decisions = decide_survivors(&forgotten_counts());
        let blocked = blocked(&decisions);
        assert_eq!(blocked.len(), 4);
        assert!(decisions.contains(&Survivor::Keep(1945)));
        assert!(decisions.contains(&Survivor::Keep(1946)));
        assert!(decisions.iter().any(|d| matches!(
            d,
            Survivor::Blocked {
                client_id: 425,
                left_count: 8,
                right_count: 169,
                ..
            }
        )));
        assert!(decisions.iter().any(|d| matches!(
            d,
            Survivor::Blocked {
                client_id: 4240,
                left_count: 17,
                right_count: 1,
                ..
            }
        )));
    }

    #[test]
    fn forgotten_otbm_matches_recorded_collision_counts() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/forgotten.otbm");
        let otbm = crate::otbm::OtbmLoader::open(&path).expect("otbm");
        let tally = otbm.tally_item_ids().expect("tally");
        let survivors = [
            (425, 177),
            (2474, 128),
            (2772, 267),
            (2773, 20),
            (3617, 2215),
            (4240, 18),
        ];
        for (id, n) in survivors {
            assert_eq!(tally.all.get(&id).copied().unwrap_or(0), n, "item {id}");
        }
    }

    #[test]
    fn unused_other_row_keeps_preferred() {
        let counts = HashMap::from([(422, 3), (4331, 0)]);
        let pair = &COLLISION_PAIRS[0];
        assert_eq!(decide_one(pair, &counts), Survivor::Keep(422));
    }
}
