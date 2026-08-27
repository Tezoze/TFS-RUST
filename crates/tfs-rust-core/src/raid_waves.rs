//! ProcessMonsterRaids — RoundNr AttackWaveQueue.
//! C++ reference: crmain.cc ProcessMonsterRaids / LoadMonsterRaids
//! Pack surface: TFS `data/raids/*.xml` + `Game.startRaid` (`raids.cpp` / `luascript.cpp`).

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::{SystemTime, UNIX_EPOCH};

use tfs_rust_common::Position;
use tfs_rust_content::raids::{RaidCatalog, RaidDefinition, RaidWave};
use tfs_rust_net::outgoing_extra::send_text_message_simple;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::return_value::ReturnValue;

/// TFS `MESSAGE_EVENT_ADVANCE` / 772 event announce (`const.h`).
const MESSAGE_EVENT_ADVANCE: u8 = 0x13;
/// TFS `MESSAGE_STATUS_WARNING` — matches 772 `TALK_ADMIN_MESSAGE` (`enums.hh`).
const MESSAGE_STATUS_WARNING: u8 = 18;
const MESSAGE_STATUS_SMALL: u8 = 20;
const MESSAGE_STATUS_DEFAULT: u8 = 21;
const MESSAGE_INFO_DESCR: u8 = 22;
const MESSAGE_STATUS_CONSOLE_BLUE: u8 = 4;
const MESSAGE_STATUS_CONSOLE_RED: u8 = 17;

/// One AttackWaveQueue entry — `crmain.cc:2021` keyed by ExecutionRound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackWave {
    pub execution_round: u32,
    pub message: Option<String>,
    pub message_class: u8,
    pub center: Position,
    pub spread: u16,
    pub monster_name: Option<String>,
    pub min_count: u16,
    pub max_count: u16,
    pub lifetime_rounds: u32,
}

impl PartialOrd for AttackWave {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AttackWave {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.execution_round
            .cmp(&other.execution_round)
            .then_with(|| self.monster_name.cmp(&other.monster_name))
            .then_with(|| self.center.x.cmp(&other.center.x))
            .then_with(|| self.center.y.cmp(&other.center.y))
    }
}

/// Catalog + priority queue + TFS `AnotherRaidIsAlreadyExecuting` latch.
#[derive(Debug, Clone, Default)]
pub struct RaidScheduler {
    pub catalog: RaidCatalog,
    queue: BinaryHeap<Reverse<AttackWave>>,
    /// Set by [`GameWorld::schedule_raid_now`] (`Raids::running`).
    pub executing: Option<String>,
}

impl RaidScheduler {
    pub fn from_catalog(catalog: RaidCatalog) -> Self {
        Self {
            catalog,
            queue: BinaryHeap::new(),
            executing: None,
        }
    }

    #[cfg(test)]
    pub fn push_wave(&mut self, wave: AttackWave) {
        self.queue.push(Reverse(wave));
    }

    /// Fire while ExecutionRound <= RoundNr (`crmain.cc` ProcessMonsterRaids).
    pub fn drain_due(&mut self, round_nr: u32) -> Vec<AttackWave> {
        let mut due = Vec::new();
        while let Some(Reverse(wave)) = self.queue.peek() {
            if wave.execution_round > round_nr {
                break;
            }
            if let Some(Reverse(wave)) = self.queue.pop() {
                due.push(wave);
            }
        }
        if self.queue.is_empty() {
            self.executing = None;
        }
        due
    }

    fn enqueue_definition(&mut self, def: &RaidDefinition, start_round: u32) {
        for wave in &def.waves {
            enqueue_xml_wave(&mut self.queue, wave, start_round);
        }
    }
}

fn ms_to_rounds(ms: u32) -> u32 {
    ms / 1000
}

fn announce_message_class(announce_type: &str) -> u8 {
    match announce_type.trim().to_ascii_lowercase().as_str() {
        "warning" => MESSAGE_STATUS_WARNING,
        "event" => MESSAGE_EVENT_ADVANCE,
        "default" => MESSAGE_STATUS_DEFAULT,
        "description" => MESSAGE_INFO_DESCR,
        "smallstatus" => MESSAGE_STATUS_SMALL,
        "blueconsole" => MESSAGE_STATUS_CONSOLE_BLUE,
        "redconsole" => MESSAGE_STATUS_CONSOLE_RED,
        _ => MESSAGE_EVENT_ADVANCE,
    }
}

fn enqueue_xml_wave(queue: &mut BinaryHeap<Reverse<AttackWave>>, wave: &RaidWave, start_round: u32) {
    match wave {
        RaidWave::Announce {
            delay_ms,
            announce_type,
            message,
        } => {
            queue.push(Reverse(AttackWave {
                execution_round: start_round.saturating_add(ms_to_rounds(*delay_ms)),
                message: Some(message.clone()),
                message_class: announce_message_class(announce_type),
                center: Position::default(),
                spread: 0,
                monster_name: None,
                min_count: 0,
                max_count: 0,
                lifetime_rounds: 0,
            }));
        }
        RaidWave::AreaSpawn {
            delay_ms,
            lifetime_ms,
            radius,
            center,
            monsters,
        } => {
            let exec = start_round.saturating_add(ms_to_rounds(*delay_ms));
            let lifetime_rounds = ms_to_rounds(*lifetime_ms);
            for m in monsters {
                queue.push(Reverse(AttackWave {
                    execution_round: exec,
                    message: None,
                    message_class: MESSAGE_EVENT_ADVANCE,
                    center: *center,
                    spread: *radius,
                    monster_name: Some(m.name.clone()),
                    min_count: m.min_amount,
                    max_count: m.max_amount,
                    lifetime_rounds,
                }));
            }
        }
        RaidWave::SingleSpawn {
            delay_ms,
            name,
            position,
        } => {
            queue.push(Reverse(AttackWave {
                execution_round: start_round.saturating_add(ms_to_rounds(*delay_ms)),
                message: None,
                message_class: MESSAGE_EVENT_ADVANCE,
                center: *position,
                spread: 0,
                monster_name: Some(name.clone()),
                min_count: 1,
                max_count: 1,
                lifetime_rounds: 0,
            }));
        }
    }
}

fn unix_now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// TFS `RETURNVALUE_*` integers for `Game.startRaid` (`constants.rs`).
pub(crate) fn raid_return_to_lua_i32(rv: ReturnValue) -> i32 {
    match rv {
        ReturnValue::NoError => 0,
        ReturnValue::NoSuchRaidExists => 61,
        ReturnValue::AnotherRaidIsAlreadyExecuting => 62,
        _ => 1,
    }
}

impl GameWorld {
    /// `Game.startRaid(name)` — queue waves at `round_nr + delay_rounds`.
    pub fn schedule_raid_now(&mut self, name: &str) -> ReturnValue {
        let Some(def) = self.raids.catalog.get(name).cloned() else {
            return ReturnValue::NoSuchRaidExists;
        };
        if self.raids.executing.is_some() {
            return ReturnValue::AnotherRaidIsAlreadyExecuting;
        }
        self.raids.executing = Some(def.name.clone());
        self.raids.enqueue_definition(&def, self.round_nr);
        ReturnValue::NoError
    }

    /// Boot interval / future-date raids — Start = round_nr + random(0, interval).
    pub fn schedule_interval_raids_at_boot(&mut self) {
        let round_nr = self.round_nr;
        let now = unix_now_secs();
        let defs: Vec<RaidDefinition> = self.raids.catalog.by_name.values().cloned().collect();
        for def in defs {
            let start = if let Some(date) = def.date_unix {
                if date <= now {
                    continue;
                }
                let delta = (date - now).clamp(0, i64::from(u32::MAX)) as u32;
                round_nr.saturating_add(delta)
            } else if let Some(interval) = def.interval_secs.filter(|s| *s > 0) {
                let jitter = self.parity_random(0, interval as i32).max(0) as u32;
                round_nr.saturating_add(jitter)
            } else {
                continue;
            };
            self.raids.enqueue_definition(&def, start);
        }
    }

    /// C++ `ProcessMonsterRaids` — drain due waves after ProcessMonsterhomes (`main.cc:355`).
    pub fn process_monster_raids(&mut self) {
        let round_nr = self.round_nr;
        self.despawn_expired_raid_monsters(round_nr);
        let due = self.raids.drain_due(round_nr);
        for wave in due {
            if let Some(ref text) = wave.message
                && !text.is_empty()
            {
                self.broadcast_raid_announce(wave.message_class, text);
            }
            if wave.monster_name.is_some() {
                self.spawn_raid_wave(&wave);
            }
        }
    }

    fn broadcast_raid_announce(&mut self, msg_class: u8, text: &str) {
        let conns: Vec<_> = self.conn_to_creature.keys().copied().collect();
        let pkt = send_text_message_simple(msg_class, text).into_bytes();
        for conn in conns {
            self.enqueue_outgoing(conn, pkt.clone());
        }
    }

    fn spawn_raid_wave(&mut self, wave: &AttackWave) {
        let Some(ref name) = wave.monster_name else {
            return;
        };
        let min = i32::from(wave.min_count);
        let max = i32::from(wave.max_count.max(wave.min_count));
        let count = self.parity_random(min, max).max(0) as u32;
        let life_end = if wave.lifetime_rounds > 0 {
            Some(self.round_nr.saturating_add(wave.lifetime_rounds))
        } else {
            None
        };
        for _ in 0..count {
            let pos = self.raid_spawn_position(wave);
            match self.lua_script_create_monster(&name, pos.x, pos.y, pos.z, true, true) {
                Ok(Some(id_bits)) => {
                    if let Some(end) = life_end {
                        set_monster_life_end(self, id_bits, end);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(monster = %name, error = %e, "raid spawn failed");
                }
            }
        }
    }

    fn raid_spawn_position(&self, wave: &AttackWave) -> Position {
        if wave.spread == 0 {
            return wave.center;
        }
        let spread = i32::from(wave.spread);
        let dx = self.parity_random(-spread, spread);
        let dy = self.parity_random(-spread, spread);
        let x = (i32::from(wave.center.x) + dx).clamp(0, i32::from(u16::MAX)) as u16;
        let y = (i32::from(wave.center.y) + dy).clamp(0, i32::from(u16::MAX)) as u16;
        Position {
            x,
            y,
            z: wave.center.z,
        }
    }

    fn despawn_expired_raid_monsters(&mut self, round_nr: u32) {
        let mut expired: Vec<CreatureId> = Vec::new();
        for (cid, kind) in self.creatures.iter() {
            if let CreatureKind::Monster(m) = kind
                && m.life_end_round.is_some_and(|end| end <= round_nr)
            {
                expired.push(cid);
            }
        }
        for cid in expired {
            self.remove_creature(cid);
        }
    }
}

fn set_monster_life_end(world: &mut GameWorld, id_bits: u64, life_end_round: u32) {
    let Some(cid) = world.resolve_creature_u64(id_bits) else {
        return;
    };
    if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
        m.life_end_round = Some(life_end_round);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_harness::{TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, lay_arena_tiles};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tfs_rust_content::monsters::{
        MonsterDatabase, MonsterDefenses, MonsterOutfit, MonsterType, MonsterTypeFlags,
    };
    use tfs_rust_content::raids::{RaidDefinition, RaidMonsterAmount, RaidWave};

    fn stub_rat() -> MonsterType {
        MonsterType {
            name: "Rat".into(),
            filename: "rat.xml".into(),
            name_description: "a rat".into(),
            race: "blood".into(),
            experience: 5,
            speed: 200,
            health_now: 20,
            health_max: 20,
            outfit: MonsterOutfit::default(),
            flags: MonsterTypeFlags::default(),
            mana_cost: 0,
            loot: Vec::new(),
            attack_spells: Vec::new(),
            defenses: MonsterDefenses {
                armor: Some(1),
                defense: Some(3),
                spells: Vec::new(),
                immunity_poison: false,
                immunity_fire: false,
                immunity_energy: false,
                immunity_life_drain: false,
                see_invisible: false,
                immunity_physical: false,
                immunity_paralyze: false,
                immunity_outfit: false,
            },
            max_summons: 0,
            summons: Vec::new(),
            talk_texts: Vec::new(),
        }
    }

    #[test]
    fn raid_wave_spawns_when_round_nr_due() {
        let mut world = beat_driven_test_world();
        let mut monsters = HashMap::new();
        monsters.insert("rat".into(), stub_rat());
        world.monsters_db = Arc::new(MonsterDatabase { monsters });
        let center = Position::new(100, 100, 7);
        lay_arena_tiles(
            &mut world.map,
            center.x,
            center.y,
            2,
            center.z,
            TEST_SYNTHETIC_GROUND_WP,
        );
        world.round_nr = 5;
        world.raids.push_wave(AttackWave {
            execution_round: 5,
            message: None,
            message_class: MESSAGE_EVENT_ADVANCE,
            center,
            spread: 0,
            monster_name: Some("Rat".to_string()),
            min_count: 1,
            max_count: 1,
            lifetime_rounds: 0,
        });
        world.process_monster_raids();
        let monsters = world
            .creatures
            .iter()
            .filter(|(_, k)| matches!(k, CreatureKind::Monster(_)))
            .count();
        assert!(
            monsters >= 1,
            "expected a raid monster after due wave, got {monsters}"
        );
    }

    #[test]
    fn start_raid_unknown_name_is_no_such_raid() {
        let mut world = beat_driven_test_world();
        assert_eq!(
            world.schedule_raid_now("does-not-exist"),
            ReturnValue::NoSuchRaidExists
        );
    }

    #[test]
    fn start_raid_while_executing_is_already_executing() {
        let mut world = beat_driven_test_world();
        world.raids.catalog.by_name.insert(
            "testraid".to_string(),
            RaidDefinition {
                name: "testraid".to_string(),
                interval_secs: None,
                date_unix: None,
                log: false,
                filename: "testraid.xml".to_string(),
                waves: vec![RaidWave::AreaSpawn {
                    delay_ms: 0,
                    lifetime_ms: 0,
                    radius: 0,
                    center: Position::new(100, 100, 7),
                    monsters: vec![RaidMonsterAmount {
                        name: "Rat".to_string(),
                        min_amount: 1,
                        max_amount: 1,
                    }],
                }],
            },
        );
        world.raids.executing = Some("other".to_string());
        assert_eq!(
            world.schedule_raid_now("testraid"),
            ReturnValue::AnotherRaidIsAlreadyExecuting
        );
    }
}
