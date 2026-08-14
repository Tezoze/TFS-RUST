//! Persist player conditions to/from the TFS `players.conditions` PropStream blob.
//!
//! Domain: TFS `Condition::serialize` / `createCondition` / `IOLoginData::savePlayer`
//! (`condition.cpp`, `iologindata.cpp`). 772 keeps the same buffs via skill Cycle fields
//! (`crplayer.cc` skill Save/Load); we keep the TFS-shaped blob so the data pack stays intact.

use tfs_rust_common::enums::ConditionType;
use tfs_rust_common::{PropStream, PropWriteStream};

use crate::condition::{ActiveCondition, ConditionData};
use crate::game_world_chat::condition_type_from_lua;

/// `ConditionAttr_t` — `condition.h`.
const CONDITIONATTR_TYPE: u8 = 1;
const CONDITIONATTR_ID: u8 = 2;
const CONDITIONATTR_TICKS: u8 = 3;
const CONDITIONATTR_HEALTHTICKS: u8 = 4;
const CONDITIONATTR_HEALTHGAIN: u8 = 5;
const CONDITIONATTR_MANATICKS: u8 = 6;
const CONDITIONATTR_MANAGAIN: u8 = 7;
const CONDITIONATTR_SPEEDDELTA: u8 = 25;
const CONDITIONATTR_FORMULA_MINA: u8 = 26;
const CONDITIONATTR_FORMULA_MINB: u8 = 27;
const CONDITIONATTR_FORMULA_MAXA: u8 = 28;
const CONDITIONATTR_FORMULA_MAXB: u8 = 29;
const CONDITIONATTR_LIGHTCOLOR: u8 = 30;
const CONDITIONATTR_LIGHTLEVEL: u8 = 31;
const CONDITIONATTR_LIGHTTICKS: u8 = 32;
const CONDITIONATTR_LIGHTINTERVAL: u8 = 33;
const CONDITIONATTR_OUTFIT: u8 = 38;
const CONDITIONATTR_ISBUFF: u8 = 40;
const CONDITIONATTR_SUBID: u8 = 41;
const CONDITIONATTR_ISAGGRESSIVE: u8 = 42;
const CONDITIONATTR_END: u8 = 254;

/// C++ `Condition::isPersistent` — `condition.cpp:301-312`.
fn is_persistent(cond: &ActiveCondition) -> bool {
    let ticks = remaining_ticks_ms(cond);
    if ticks <= 0 {
        return false;
    }
    // `CONDITIONID_DEFAULT` (-1 as u32) or `CONDITIONID_COMBAT` (0), or muted.
    cond.id == 0 || cond.id == u32::MAX || cond.ctype == ConditionType::Muted
}

fn remaining_ticks_ms(cond: &ActiveCondition) -> i32 {
    if let Some(rounds) = cond.timer_rounds_left {
        return rounds.saturating_mul(1000).max(0);
    }
    match &cond.data {
        ConditionData::Generic { ticks } => (*ticks).max(0),
        ConditionData::Regeneration {
            health_ticks_ms,
            mana_ticks_ms,
            ..
        } => {
            // Prefer an explicit timer; regen often has long ticks in Generic path.
            (*health_ticks_ms).max(*mana_ticks_ms) as i32
        }
        _ => 0,
    }
}

/// Map runtime enum → TFS `ConditionType_t` bit flag for the blob.
fn condition_type_to_tfs_flag(ctype: ConditionType) -> u32 {
    match ctype {
        ConditionType::None => 0,
        ConditionType::Poison => 1 << 0,
        ConditionType::Fire => 1 << 1,
        ConditionType::Energy => 1 << 2,
        ConditionType::Bleeding => 1 << 3,
        ConditionType::Haste => 1 << 4,
        ConditionType::Paralyze => 1 << 5,
        ConditionType::Outfit => 1 << 6,
        ConditionType::Invisible => 1 << 7,
        ConditionType::Light => 1 << 8,
        ConditionType::ManaShield => 1 << 9,
        ConditionType::Infight => 1 << 10,
        ConditionType::Drunk => 1 << 11,
        ConditionType::ExhaustWeapon => 1 << 12,
        ConditionType::Regeneration => 1 << 13,
        // Our runtime `Muted` maps from Lua `1<<14` historically; TFS `CONDITION_MUTED` is `1<<16`.
        // Persist with the flag our `condition_type_from_lua` understands so round-trips work.
        ConditionType::Muted => 1 << 14,
        ConditionType::ChannelMutedTicks => 1 << 15,
        ConditionType::YellTicks => 1 << 16,
        ConditionType::Attributes => 1 << 17,
        ConditionType::Freezing => 1 << 20,
        ConditionType::Dazzled => 1 << 21,
        ConditionType::Cursed => 1 << 22,
        ConditionType::ExhaustCombat => 1 << 23,
        ConditionType::ExhaustHeal => 1 << 24,
        ConditionType::ExhaustGroup | ConditionType::Pz => 0,
    }
}

fn write_base_header(w: &mut PropWriteStream, cond: &ActiveCondition, ticks: i32) {
    w.write_u8(CONDITIONATTR_TYPE);
    w.write_u32(condition_type_to_tfs_flag(cond.ctype));
    w.write_u8(CONDITIONATTR_ID);
    w.write_u32(cond.id);
    w.write_u8(CONDITIONATTR_TICKS);
    w.write_u32(ticks.max(0) as u32);
    w.write_u8(CONDITIONATTR_ISBUFF);
    w.write_u8(0);
    w.write_u8(CONDITIONATTR_SUBID);
    w.write_u32(cond.sub_id);
    w.write_u8(CONDITIONATTR_ISAGGRESSIVE);
    w.write_u8(0);
}

fn write_one(w: &mut PropWriteStream, cond: &ActiveCondition) {
    let ticks = remaining_ticks_ms(cond);
    write_base_header(w, cond, ticks);
    match &cond.data {
        ConditionData::Speed { flat_delta } => {
            w.write_u8(CONDITIONATTR_SPEEDDELTA);
            w.write_i32(*flat_delta);
            w.write_u8(CONDITIONATTR_FORMULA_MINA);
            w.write_f32(0.0);
            w.write_u8(CONDITIONATTR_FORMULA_MINB);
            w.write_f32(0.0);
            w.write_u8(CONDITIONATTR_FORMULA_MAXA);
            w.write_f32(0.0);
            w.write_u8(CONDITIONATTR_FORMULA_MAXB);
            w.write_f32(0.0);
        }
        ConditionData::Light { level, color } => {
            w.write_u8(CONDITIONATTR_LIGHTCOLOR);
            w.write_i32(i32::from(*color));
            w.write_u8(CONDITIONATTR_LIGHTLEVEL);
            w.write_i32(i32::from(*level));
            w.write_u8(CONDITIONATTR_LIGHTTICKS);
            w.write_i32(ticks);
            w.write_u8(CONDITIONATTR_LIGHTINTERVAL);
            w.write_i32(1000);
        }
        ConditionData::Outfit {
            look_type,
            look_type_ex,
        } => {
            w.write_u8(CONDITIONATTR_OUTFIT);
            // `Outfit_t` — `enums.h:500-509` (raw struct write).
            w.write_u16((*look_type).clamp(0, u16::MAX as i32) as u16);
            w.write_u16(*look_type_ex);
            w.write_u16(0); // lookMount
            w.write_u8(0); // head
            w.write_u8(0);
            w.write_u8(0);
            w.write_u8(0);
            w.write_u8(0); // addons
            w.write_u8(0); // struct padding to sizeof(Outfit_t)==12
        }
        ConditionData::Regeneration {
            health_gain,
            health_ticks_ms,
            mana_gain,
            mana_ticks_ms,
            ..
        } => {
            w.write_u8(CONDITIONATTR_HEALTHTICKS);
            w.write_u32(*health_ticks_ms);
            w.write_u8(CONDITIONATTR_HEALTHGAIN);
            w.write_u32((*health_gain).max(0) as u32);
            w.write_u8(CONDITIONATTR_MANATICKS);
            w.write_u32(*mana_ticks_ms);
            w.write_u8(CONDITIONATTR_MANAGAIN);
            w.write_u32((*mana_gain).max(0) as u32);
        }
        _ => {}
    }
    w.write_u8(CONDITIONATTR_END);
}

/// Serialize persistent conditions — TFS `IOLoginData::savePlayer` conditions loop.
pub fn serialize_conditions(conds: &[ActiveCondition]) -> Vec<u8> {
    let mut w = PropWriteStream::new();
    for cond in conds {
        if is_persistent(cond) {
            write_one(&mut w, cond);
        }
    }
    w.finish()
}

struct PartialCond {
    ctype: ConditionType,
    id: u32,
    ticks: i32,
    sub_id: u32,
    speed_delta: Option<i32>,
    light_level: Option<u8>,
    light_color: Option<u8>,
    look_type: Option<i32>,
    look_type_ex: Option<u16>,
    health_gain: Option<i32>,
    health_ticks_ms: Option<u32>,
    mana_gain: Option<i32>,
    mana_ticks_ms: Option<u32>,
}

impl PartialCond {
    fn into_active(self) -> Option<ActiveCondition> {
        if self.ctype == ConditionType::None || self.ticks <= 0 {
            return None;
        }
        let rounds = Some((self.ticks.max(1) + 999) / 1000);
        let data = match self.ctype {
            ConditionType::Haste | ConditionType::Paralyze => ConditionData::Speed {
                flat_delta: self.speed_delta.unwrap_or(0),
            },
            ConditionType::Light => ConditionData::Light {
                level: self.light_level.unwrap_or(0),
                color: self.light_color.unwrap_or(0),
            },
            ConditionType::Outfit => ConditionData::Outfit {
                look_type: self.look_type.unwrap_or(0),
                look_type_ex: self.look_type_ex.unwrap_or(0),
            },
            ConditionType::Regeneration => ConditionData::Regeneration {
                health_gain: self.health_gain.unwrap_or(0),
                health_ticks_ms: self.health_ticks_ms.unwrap_or(0),
                mana_gain: self.mana_gain.unwrap_or(0),
                mana_ticks_ms: self.mana_ticks_ms.unwrap_or(0),
                health_elapsed_ms: 0,
                mana_elapsed_ms: 0,
            },
            ConditionType::Invisible
            | ConditionType::ManaShield
            | ConditionType::Infight
            | ConditionType::Drunk
            | ConditionType::Muted
            | ConditionType::ChannelMutedTicks
            | ConditionType::YellTicks
            | ConditionType::Pz => ConditionData::Generic { ticks: self.ticks },
            // DoTs / attributes: keep a timer so they expire; damage payload may be incomplete.
            _ => ConditionData::Generic { ticks: self.ticks },
        };
        Some(ActiveCondition {
            id: self.id,
            sub_id: self.sub_id,
            ctype: self.ctype,
            data,
            timer_rounds_left: rounds,
            skill_count: 0,
            skill_max_count: 0,
        })
    }
}

/// Deserialize — TFS `IOLoginData::loadPlayer` conditions PropStream loop.
#[allow(clippy::while_let_loop)] // PropStream: `break` on header/payload errors, not only `read_u8` EOF.
pub fn deserialize_conditions(blob: &[u8]) -> Vec<ActiveCondition> {
    if blob.is_empty() {
        return Vec::new();
    }
    let mut stream = PropStream::new(blob);
    let mut out = Vec::new();
    loop {
        // `Condition::createCondition(PropStream)` — header must start with TYPE.
        let Ok(attr) = stream.read_u8() else {
            break;
        };
        if attr != CONDITIONATTR_TYPE {
            break;
        }
        let Ok(type_flag) = stream.read_u32() else {
            break;
        };
        let Ok(attr) = stream.read_u8() else {
            break;
        };
        if attr != CONDITIONATTR_ID {
            break;
        }
        let Ok(id) = stream.read_u32() else {
            break;
        };
        let Ok(attr) = stream.read_u8() else {
            break;
        };
        if attr != CONDITIONATTR_TICKS {
            break;
        }
        let Ok(ticks_u) = stream.read_u32() else {
            break;
        };
        let Ok(attr) = stream.read_u8() else {
            break;
        };
        if attr != CONDITIONATTR_ISBUFF {
            break;
        }
        let Ok(_buff) = stream.read_u8() else {
            break;
        };
        let Ok(attr) = stream.read_u8() else {
            break;
        };
        if attr != CONDITIONATTR_SUBID {
            break;
        }
        let Ok(sub_id) = stream.read_u32() else {
            break;
        };
        let Ok(attr) = stream.read_u8() else {
            break;
        };
        if attr != CONDITIONATTR_ISAGGRESSIVE {
            break;
        }
        let Ok(_agg) = stream.read_u8() else {
            break;
        };

        let mut partial = PartialCond {
            ctype: condition_type_from_lua(type_flag as i32),
            id,
            ticks: ticks_u as i32,
            sub_id,
            speed_delta: None,
            light_level: None,
            light_color: None,
            look_type: None,
            look_type_ex: None,
            health_gain: None,
            health_ticks_ms: None,
            mana_gain: None,
            mana_ticks_ms: None,
        };

        // `Condition::unserialize` until END — may re-read TYPE/ID/TICKS from serialize.
        loop {
            let Ok(a) = stream.read_u8() else {
                break;
            };
            if a == CONDITIONATTR_END {
                break;
            }
            match a {
                CONDITIONATTR_TYPE => {
                    if let Ok(v) = stream.read_u32() {
                        partial.ctype = condition_type_from_lua(v as i32);
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_ID => {
                    if let Ok(v) = stream.read_u32() {
                        partial.id = v;
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_TICKS => {
                    if let Ok(v) = stream.read_u32() {
                        partial.ticks = v as i32;
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_ISBUFF | CONDITIONATTR_ISAGGRESSIVE => {
                    if stream.read_u8().is_err() {
                        break;
                    }
                }
                CONDITIONATTR_SUBID => {
                    if let Ok(v) = stream.read_u32() {
                        partial.sub_id = v;
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_SPEEDDELTA => {
                    if let Ok(v) = stream.read_i32() {
                        partial.speed_delta = Some(v);
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_FORMULA_MINA
                | CONDITIONATTR_FORMULA_MINB
                | CONDITIONATTR_FORMULA_MAXA
                | CONDITIONATTR_FORMULA_MAXB => {
                    if stream.read_f32().is_err() {
                        break;
                    }
                }
                CONDITIONATTR_LIGHTCOLOR => {
                    if let Ok(v) = stream.read_i32() {
                        partial.light_color = Some(v.clamp(0, 255) as u8);
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_LIGHTLEVEL => {
                    if let Ok(v) = stream.read_i32() {
                        partial.light_level = Some(v.clamp(0, 255) as u8);
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_LIGHTTICKS | CONDITIONATTR_LIGHTINTERVAL => {
                    if stream.read_i32().is_err() {
                        break;
                    }
                }
                CONDITIONATTR_OUTFIT => {
                    let Ok(look_type) = stream.read_u16() else {
                        break;
                    };
                    let Ok(look_type_ex) = stream.read_u16() else {
                        break;
                    };
                    let _ = stream.read_u16(); // mount
                    let _ = stream.read_u8();
                    let _ = stream.read_u8();
                    let _ = stream.read_u8();
                    let _ = stream.read_u8();
                    let _ = stream.read_u8();
                    let _ = stream.read_u8(); // struct padding
                    partial.look_type = Some(i32::from(look_type));
                    partial.look_type_ex = Some(look_type_ex);
                }
                CONDITIONATTR_HEALTHTICKS => {
                    if let Ok(v) = stream.read_u32() {
                        partial.health_ticks_ms = Some(v);
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_HEALTHGAIN => {
                    if let Ok(v) = stream.read_u32() {
                        partial.health_gain = Some(v as i32);
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_MANATICKS => {
                    if let Ok(v) = stream.read_u32() {
                        partial.mana_ticks_ms = Some(v);
                    } else {
                        break;
                    }
                }
                CONDITIONATTR_MANAGAIN => {
                    if let Ok(v) = stream.read_u32() {
                        partial.mana_gain = Some(v as i32);
                    } else {
                        break;
                    }
                }
                _ => {
                    // Unknown attr — abort this condition rather than desync the stream.
                    tracing::warn!(attr = a, "condition blob: skipping unknown attr");
                    break;
                }
            }
        }

        if let Some(active) = partial.into_active() {
            out.push(active);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_mana_shield_haste_invis() {
        let conds = vec![
            ActiveCondition {
                id: 0,
                sub_id: 0,
                ctype: ConditionType::ManaShield,
                data: ConditionData::Generic { ticks: 200_000 },
                timer_rounds_left: Some(200),
                skill_count: 0,
                skill_max_count: 0,
            },
            ActiveCondition {
                id: 0,
                sub_id: 0,
                ctype: ConditionType::Haste,
                data: ConditionData::Speed { flat_delta: 60 },
                timer_rounds_left: Some(30),
                skill_count: 0,
                skill_max_count: 0,
            },
            ActiveCondition {
                id: 0,
                sub_id: 0,
                ctype: ConditionType::Invisible,
                data: ConditionData::Generic { ticks: 200_000 },
                timer_rounds_left: Some(200),
                skill_count: 0,
                skill_max_count: 0,
            },
        ];
        let blob = serialize_conditions(&conds);
        assert!(!blob.is_empty());
        let loaded = deserialize_conditions(&blob);
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].ctype, ConditionType::ManaShield);
        assert_eq!(loaded[0].timer_rounds_left, Some(200));
        assert_eq!(loaded[1].ctype, ConditionType::Haste);
        assert_eq!(loaded[1].data, ConditionData::Speed { flat_delta: 60 });
        assert_eq!(loaded[1].timer_rounds_left, Some(30));
        assert_eq!(loaded[2].ctype, ConditionType::Invisible);
        assert_eq!(loaded[2].timer_rounds_left, Some(200));
    }

    #[test]
    fn skips_zero_timer() {
        let conds = vec![ActiveCondition {
            id: 0,
            sub_id: 0,
            ctype: ConditionType::ManaShield,
            data: ConditionData::Generic { ticks: 0 },
            timer_rounds_left: Some(0),
            skill_count: 0,
            skill_max_count: 0,
        }];
        assert!(serialize_conditions(&conds).is_empty());
    }
}
