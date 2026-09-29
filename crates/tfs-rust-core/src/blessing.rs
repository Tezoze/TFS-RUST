//! Blessing bits kept in step with the 772 quest flags.
//!
//! Corpus: `TPlayer::Death` (`crplayer.cc:343-348`) counts and clears quests 101–105.
//! The bitfield is the later-codec store. Each counted quest sets one named bit.
//! Quest 199 (Kawill) is a gate for Pydar and is not a bit.

use tfs_rust_content::npcs::Blessing;

use crate::creature::Player;

/// Or in a bit for every counted blessing whose quest value is non-zero.
pub fn fold_quest_blessings(bits: i8, storage: &[(u32, i32)]) -> i8 {
    let mut bits = bits;
    for blessing in Blessing::COUNTED {
        let Some(quest) = blessing.quest_id() else {
            continue;
        };
        if storage.iter().any(|(id, value)| *id == quest && *value > 0) {
            bits |= blessing.mask();
        }
    }
    bits
}

/// Set or clear the bit for a blessing quest. Other storage keys are ignored.
pub fn apply_quest_to_bits(bits: i8, storage_id: u32, value: i32) -> i8 {
    let Some(blessing) = Blessing::from_quest(storage_id) else {
        return bits;
    };
    if value > 0 {
        bits | blessing.mask()
    } else {
        bits & !blessing.mask()
    }
}

/// 772 death: drop the five counted blessings. Twist and quest 199 stay.
pub fn consume_counted_blessings(player: &mut Player) {
    for blessing in Blessing::COUNTED {
        player.blessings &= !blessing.mask();
    }
    if let Some(persist) = player.persist.as_mut() {
        persist.player_row.blessings = player.blessings;
        for slot in persist.storage.iter_mut() {
            if Blessing::from_quest(slot.0).is_some() {
                slot.1 = 0;
            }
        }
    }
}

/// After a later-codec bit clear, zero the quest of each counted blessing whose bit is off.
pub fn clear_quests_for_missing_bits(player: &mut Player) {
    let bits = player.blessings;
    if let Some(persist) = player.persist.as_mut() {
        persist.player_row.blessings = bits;
        for blessing in Blessing::COUNTED {
            if bits & blessing.mask() != 0 {
                continue;
            }
            let Some(quest) = blessing.quest_id() else {
                continue;
            };
            if let Some(slot) = persist.storage.iter_mut().find(|(id, _)| *id == quest) {
                slot.1 = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_map_to_bits_and_quests() {
        assert_eq!(Blessing::from_word("shielding").unwrap().bit(), 0);
        assert_eq!(Blessing::from_word("embrace").unwrap().bit(), 1);
        assert_eq!(Blessing::from_word("suns").unwrap().bit(), 2);
        assert_eq!(Blessing::from_word("spark").unwrap().bit(), 3);
        assert_eq!(Blessing::from_word("solitude").unwrap().bit(), 4);
        assert_eq!(Blessing::from_word("twist").unwrap().bit(), 5);
        assert_eq!(Blessing::Shielding.quest_id(), Some(104));
        assert_eq!(Blessing::Embrace.quest_id(), Some(105));
        assert_eq!(Blessing::Suns.quest_id(), Some(103));
        assert_eq!(Blessing::Spark.quest_id(), Some(102));
        assert_eq!(Blessing::Solitude.quest_id(), Some(101));
        assert_eq!(Blessing::Twist.quest_id(), None);
        assert_eq!(Blessing::from_quest(199), None);
    }

    #[test]
    fn spark_quest_sets_only_the_spark_bit() {
        let bits = apply_quest_to_bits(0, 102, 1);
        assert_eq!(bits, Blessing::Spark.mask());
        assert_eq!(apply_quest_to_bits(bits, 199, 1), bits);
        assert_eq!(
            apply_quest_to_bits(bits, 103, 3),
            bits | Blessing::Suns.mask()
        );
        assert_eq!(apply_quest_to_bits(bits, 102, 0), 0);
    }
}
