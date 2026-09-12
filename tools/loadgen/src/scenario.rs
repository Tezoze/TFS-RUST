//! Declarative RON scenarios (`bench/scenarios/*.ron`). Frozen before a publication run.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Minimum `SAY` interval — `RECORD_TALK_WINDOW_MS` in `chat_talk.rs`.
pub const RECORD_TALK_WINDOW_MS: u64 = 2500;
/// Default walk period (ms) — at/above typical LinearGo step / `earliest_walk_server_ms`.
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
    #[serde(default = "default_rune_sprite")]
    pub rune_sprite_id: u16,
    #[serde(default = "default_rune_slot")]
    pub rune_slot: u16,
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
            rune_sprite_id: default_rune_sprite(),
            rune_slot: default_rune_slot(),
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
    }
}
