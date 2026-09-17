//! Declarative RON scenarios (`bench/scenarios/*.ron`). Frozen before a publication run.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Minimum `SAY` interval — `RECORD_TALK_WINDOW_MS` in `chat_talk.rs`.
pub const RECORD_TALK_WINDOW_MS: u64 = 2500;
/// Frozen `mixed_300.ron` walk period (ms). Level-50 sorcerer on grass is ~500 ms
/// LinearGo (`voc_base 70 + 49`, GetSpeed 318, ceil-to-beat 50). New scenarios
/// should set `walk_period_ms: 500`; do not edit mixed_300.
pub const DEFAULT_WALK_PERIOD_MS: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum RoleKind {
    Walker,
    Melee,
    Caster,
    Rune,
    AoeRune,
    Noise,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleWeight {
    pub kind: RoleKind,
    pub weight: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    pub name: String,
    #[serde(default = "default_bots")]
    pub bots: usize,
    #[serde(default = "default_duration")]
    pub duration_s: u64,
    #[serde(default)]
    pub warmup_s: u64,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_walk_period")]
    pub walk_period_ms: u64,
    #[serde(default = "default_say_period")]
    pub say_period_ms: u64,
    pub roles: Vec<RoleWeight>,
    #[serde(default = "default_spell")]
    pub spell_words: String,
    /// If non-empty, casters pick uniformly from this instead of `spell_words`.
    #[serde(default)]
    pub spell_words_pool: Vec<String>,
    /// Random SAY texts (Noise always; Caster mixes with spells). Empty → `"hi"`.
    #[serde(default)]
    pub chat_messages: Vec<String>,
    #[serde(default = "default_rune_sprite")]
    pub rune_sprite_id: u16,
    #[serde(default = "default_rune_slot")]
    pub rune_slot: u16,
    /// Sudden Death server id — client look id is resolved from `TFS_ITEMS_OTB`.
    #[serde(default = "default_rune_server")]
    pub rune_server_id: u16,
    /// Great Fireball server id (area rune; `needTarget` is false on both packs).
    #[serde(default = "default_aoe_rune_server")]
    pub aoe_rune_server_id: u16,
    /// Left-hand slot for the AoE rune so mixed bots can hold SD in ammo (10).
    #[serde(default = "default_aoe_rune_slot")]
    pub aoe_rune_slot: u16,
    /// Expected `0x83` effect id for AoE-rune acks. GFB combat sets
    /// `COMBAT_PARAM_EFFECT = CONST_ME_FIREAREA` (7); override if the pack
    /// changes. `None` disables effect matching (any effect at the tile acks).
    #[serde(default = "default_aoe_rune_effect")]
    pub aoe_rune_effect: Option<u8>,
    /// Repo-relative CSV (`bench/waypoints/…`). `None` → random cardinals.
    #[serde(default)]
    pub waypoint_file: Option<String>,
}

fn default_bots() -> usize {
    1
}
fn default_duration() -> u64 {
    30
}
fn default_seed() -> u64 {
    42
}
fn default_walk_period() -> u64 {
    DEFAULT_WALK_PERIOD_MS
}
fn default_say_period() -> u64 {
    RECORD_TALK_WINDOW_MS
}
fn default_spell() -> String {
    "exori vis".into()
}
fn default_rune_sprite() -> u16 {
    3155
}
fn default_rune_slot() -> u16 {
    10
}
fn default_rune_server() -> u16 {
    2268
}
fn default_aoe_rune_server() -> u16 {
    2304
}
fn default_aoe_rune_slot() -> u16 {
    6
}

/// GFB `CONST_ME_FIREAREA` (`combat_enums.rs`, `great_fireball_rune.lua`).
fn default_aoe_rune_effect() -> Option<u8> {
    Some(7)
}

impl Scenario {
    pub fn load_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read scenario {}", path.display()))?;
        let s: Self = ron::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        anyhow::ensure!(!s.roles.is_empty(), "scenario has no roles");
        anyhow::ensure!(
            s.say_period_ms >= RECORD_TALK_WINDOW_MS,
            "say_period_ms must be >= {RECORD_TALK_WINDOW_MS} (RecordTalk window)"
        );
        if let Some(ref file) = s.waypoint_file {
            let path = crate::waypoints::resolve_waypoint_path(file);
            anyhow::ensure!(
                path.is_file(),
                "waypoint_file not found: {file} ({})",
                path.display()
            );
            let _ = crate::waypoints::load_csv(&path)?;
        }
        Ok(s)
    }

    pub fn walker_spike(duration_s: u64, walk_period_ms: u64, seed: u64) -> Self {
        Self {
            name: "walk_ns".into(),
            bots: 1,
            duration_s,
            warmup_s: 0,
            seed,
            walk_period_ms,
            say_period_ms: RECORD_TALK_WINDOW_MS,
            roles: vec![RoleWeight {
                kind: RoleKind::Walker,
                weight: 1.0,
            }],
            spell_words: default_spell(),
            spell_words_pool: Vec::new(),
            chat_messages: Vec::new(),
            rune_sprite_id: default_rune_sprite(),
            rune_slot: default_rune_slot(),
            rune_server_id: default_rune_server(),
            aoe_rune_server_id: default_aoe_rune_server(),
            aoe_rune_slot: default_aoe_rune_slot(),
            aoe_rune_effect: default_aoe_rune_effect(),
            waypoint_file: None,
        }
    }

    pub fn pick_role(&self, bot_index: usize, rng: &mut BotRng) -> RoleKind {
        let total: f64 = self.roles.iter().map(|r| r.weight.max(0.0)).sum();
        if total <= 0.0 {
            return self.roles[0].kind;
        }
        // Stable assignment: hash bot index through weights rather than one draw that
        // would make every bot the same when they share a stream start.
        let _ = bot_index;
        let mut x = rng.next_f64() * total;
        for r in &self.roles {
            x -= r.weight.max(0.0);
            if x <= 0.0 {
                return r.kind;
            }
        }
        self.roles
            .last()
            .map(|r| r.kind)
            .unwrap_or(RoleKind::Walker)
    }

    fn pick_from<'a>(rng: &mut BotRng, items: &'a [String]) -> &'a str {
        &items[rng.next_u64() as usize % items.len()]
    }

    pub fn spell_say_text(&self, rng: &mut BotRng) -> String {
        if self.spell_words_pool.is_empty() {
            self.spell_words.clone()
        } else {
            Self::pick_from(rng, &self.spell_words_pool).to_string()
        }
    }

    pub fn noise_say_text(&self, rng: &mut BotRng) -> String {
        if self.chat_messages.is_empty() {
            "hi".into()
        } else {
            Self::pick_from(rng, &self.chat_messages).to_string()
        }
    }

    /// Caster SAY: ~40% chat when `chat_messages` is set, else a spell.
    /// Second value is `true` when the text is a spell (enqueue `SpellRune`).
    pub fn caster_say(&self, rng: &mut BotRng) -> (String, bool) {
        if !self.chat_messages.is_empty() && rng.next_f64() < 0.4 {
            (self.noise_say_text(rng), false)
        } else {
            (self.spell_say_text(rng), true)
        }
    }

    pub fn caster_say_text(&self, rng: &mut BotRng) -> String {
        self.caster_say(rng).0
    }
}

/// Xorshift64* — per-bot seeded (`seed.wrapping_add(bot_index)`).
#[derive(Debug, Clone)]
pub struct BotRng {
    state: u64,
}

impl BotRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    pub fn gen_range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % u64::from(hi - lo)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walker_ron_roundtrip() {
        let text = r#"(
            name: "walker",
            bots: 1,
            duration_s: 10,
            roles: [(kind: Walker, weight: 1.0)],
        )"#;
        let s: Scenario = ron::from_str(text).expect("ron");
        assert_eq!(s.name, "walker");
        assert_eq!(s.walk_period_ms, DEFAULT_WALK_PERIOD_MS);
        assert_eq!(s.say_period_ms, RECORD_TALK_WINDOW_MS);
    }

    #[test]
    fn mixed_300_file_loads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bench/scenarios/mixed_300.ron");
        let s = Scenario::load_path(&path).expect("mixed_300");
        assert_eq!(s.bots, 300);
        assert_eq!(s.roles.len(), 6);
        assert!(s.waypoint_file.is_none());
    }

    #[test]
    fn walker_loop_file_loads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bench/scenarios/walker_loop.ron");
        let s = Scenario::load_path(&path).expect("walker_loop");
        assert_eq!(s.bots, 25);
        assert_eq!(s.walk_period_ms, 500);
        assert!(s.waypoint_file.is_some());
    }

    #[test]
    fn clustered_hunt_file_loads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bench/scenarios/clustered_hunt.ron");
        let s = Scenario::load_path(&path).expect("clustered_hunt");
        assert_eq!(s.bots, 50);
        assert_eq!(s.walk_period_ms, 500);
        assert!(s.waypoint_file.as_deref().unwrap().contains("cyclops"));
    }

    #[test]
    fn walker_isolation_file_loads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bench/scenarios/walker.ron");
        let s = Scenario::load_path(&path).expect("walker");
        assert_eq!(s.bots, 5);
        assert_eq!(s.walk_period_ms, 500);
        assert!(s.waypoint_file.as_deref().unwrap().contains("cyclops"));
    }

    #[test]
    fn ms_swarm_file_loads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bench/scenarios/ms_swarm.ron");
        let s = Scenario::load_path(&path).expect("ms_swarm");
        assert_eq!(s.bots, 1000);
        assert_eq!(s.roles.len(), 1);
        assert_eq!(s.roles[0].kind, RoleKind::Caster);
        assert!(
            s.spell_words_pool
                .iter()
                .any(|w| w.contains("exevo gran mas vis"))
        );
        assert!(s.spell_words_pool.iter().any(|w| w.contains("exura vita")));
        assert!(s.chat_messages.iter().any(|m| m.contains("bot swarm")));
        assert!(s.waypoint_file.is_none());
    }

    #[test]
    fn ue_isolation_file_loads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bench/scenarios/ue_isolation.ron");
        let s = Scenario::load_path(&path).expect("ue_isolation");
        assert_eq!(s.bots, 5);
        assert!(s.chat_messages.is_empty());
        assert_eq!(s.spell_words_pool.len(), 1);
        assert!(s.spell_words_pool[0].contains("exevo gran mas vis"));
        assert!(s.waypoint_file.as_deref().unwrap().contains("cyclops"));
    }

    #[test]
    fn caster_say_mixes_pool_and_chat() {
        let s: Scenario = ron::from_str(
            r#"(
            name: "x",
            roles: [(kind: Caster, weight: 1.0)],
            spell_words_pool: ["exevo gran mas vis", "exura vita"],
            chat_messages: ["we are a bot swarm"],
        )"#,
        )
        .expect("ron");
        let mut rng = BotRng::new(1);
        let mut saw_spell = false;
        let mut saw_chat = false;
        for _ in 0..80 {
            let (t, spell) = s.caster_say(&mut rng);
            if t.contains("exevo") || t.contains("exura") {
                saw_spell = true;
                assert!(spell, "{t}");
            }
            if t.contains("bot swarm") {
                saw_chat = true;
                assert!(!spell, "{t}");
            }
        }
        assert!(saw_spell && saw_chat);
    }

    #[test]
    fn noise_defaults_to_hi_without_chat_pool() {
        let s: Scenario = ron::from_str(
            r#"(
            name: "x",
            roles: [(kind: Noise, weight: 1.0)],
        )"#,
        )
        .expect("ron");
        let mut rng = BotRng::new(7);
        assert_eq!(s.noise_say_text(&mut rng), "hi");
    }
}
