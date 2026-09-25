//! `hdrhistogram` walk-ack and spell/rune latency. Intended-time, not send-time.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use hdrhistogram::Histogram;

const SIGFIGS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Correlate {
    Walk,
    SpellRune,
}

pub struct LatencySet {
    walk: Histogram<u64>,
    spell: Histogram<u64>,
    outstanding_walk: Vec<Instant>,
    outstanding_spell: Vec<(Instant, tfs_rust_common::Position, Option<u8>)>,
    walk_rejections: u64,
    spell_rejections: u64,
    /// `0xB4` walk text already retired; the following `0xB5` is the paired Snapback.
    suppress_paired_snapback: bool,
    /// Next `0xB4` spell reject belongs to a superseded cast (`on_send` cap-1).
    suppress_paired_spell_reject: bool,
}

impl LatencySet {
    pub fn new() -> Result<Self> {
        Ok(Self {
            walk: Histogram::new(SIGFIGS).map_err(|e| anyhow!("walk histogram: {e}"))?,
            spell: Histogram::new(SIGFIGS).map_err(|e| anyhow!("spell histogram: {e}"))?,
            outstanding_walk: Vec::new(),
            outstanding_spell: Vec::new(),
            walk_rejections: 0,
            spell_rejections: 0,
            suppress_paired_snapback: false,
            suppress_paired_spell_reject: false,
        })
    }

    /// Record an intended send. FIFO per correlation type.
    pub fn on_send(
        &mut self,
        kind: Correlate,
        intended: Instant,
        tile: Option<tfs_rust_common::Position>,
        effect: Option<u8>,
    ) {
        match kind {
            Correlate::Walk => {
                // Open-loop may send the next `CGo` before the previous acks.
                // Server `CGoDirection` `ToDoClear` + `SendSnapback` replaces the
                // pending Go (`receiving.cc:120-199`); keep only the latest intended
                // so FIFO depth is not (period × stuck-seconds). The following `0xB5`
                // is suppressed. `20260919T224609Z` p99 18 s; unpaired `0xB4` Sorry
                // retire made it worse (`20260919T225517Z`). Do not clear suppress
                // here — a same-payload `WalkRejected` may still need the following
                // `0xB5` coalesced (`lesson 497`).
                if !self.outstanding_walk.is_empty() {
                    let _ = self.pop_walk_head();
                    self.suppress_paired_snapback = true;
                }
                self.outstanding_walk.push(intended);
            }
            Correlate::SpellRune => {
                // Same open-loop pile as walk: `say_period_ms=2500` leaves the
                // oldest same-tile head to age until a later `0x83`
                // (`20260919T234241Z` spell p99 25 s). Keep the latest intended.
                // The following `0xB4` (mana/PZ for the dropped cast) is
                // suppressed; a later `0x83` still matches this send.
                if let Some(pos) = tile {
                    if !self.outstanding_spell.is_empty() {
                        let _ = self.outstanding_spell.remove(0);
                        self.suppress_paired_spell_reject = true;
                    }
                    self.outstanding_spell.push((intended, pos, effect));
                }
            }
        }
    }

    pub fn on_walk_ack(&mut self, now: Instant) {
        self.suppress_paired_snapback = false;
        if let Some(intended) = self.pop_walk_head() {
            record(&mut self.walk, now.saturating_duration_since(intended));
        }
    }

    /// Retire the oldest outstanding walk as a rejection (`0xB5`), not a sample.
    pub fn on_walk_cancel(&mut self) {
        if self.suppress_paired_snapback {
            self.suppress_paired_snapback = false;
            return;
        }
        if self.pop_walk_head().is_some() {
            self.walk_rejections += 1;
        }
    }

    /// Same-payload walk bump (`0xB4` + trailing `0xB5`). The following
    /// [`Self::on_walk_cancel`] is the paired Snapback.
    pub fn on_walk_text_reject(&mut self) {
        if self.pop_walk_head().is_some() {
            self.walk_rejections += 1;
            self.suppress_paired_snapback = true;
        }
    }

    fn pop_walk_head(&mut self) -> Option<Instant> {
        if self.outstanding_walk.is_empty() {
            None
        } else {
            Some(self.outstanding_walk.remove(0))
        }
    }

    pub fn on_magic_effect(&mut self, now: Instant, pos: tfs_rust_common::Position, effect: u8) {
        if let Some(idx) = self
            .outstanding_spell
            .iter()
            .position(|(_, p, want)| *p == pos && want.is_none_or(|e| e == effect))
        {
            self.suppress_paired_spell_reject = false;
            let (intended, _, _) = self.outstanding_spell.remove(idx);
            record(&mut self.spell, now.saturating_duration_since(intended));
        }
    }

    /// Retire the oldest outstanding spell/rune as a rejection (`0xB4` cancel
    /// text), not a sample. No-op with an empty queue so login MOTD / broadcast
    /// text never counts. After cap-1 drop, the next reject is the superseded
    /// cast and is ignored.
    pub fn on_spell_reject(&mut self) {
        if self.suppress_paired_spell_reject {
            self.suppress_paired_spell_reject = false;
            return;
        }
        if !self.outstanding_spell.is_empty() {
            self.outstanding_spell.remove(0);
            self.spell_rejections += 1;
        }
    }

    pub fn outstanding_count(&self) -> u64 {
        (self.outstanding_walk.len() + self.outstanding_spell.len()) as u64
    }

    pub fn add(&mut self, other: &LatencySet) -> Result<()> {
        self.walk
            .add(&other.walk)
            .map_err(|e| anyhow!("merge walk: {e}"))?;
        self.spell
            .add(&other.spell)
            .map_err(|e| anyhow!("merge spell: {e}"))?;
        self.outstanding_walk
            .extend_from_slice(&other.outstanding_walk);
        self.outstanding_spell
            .extend_from_slice(&other.outstanding_spell);
        self.walk_rejections += other.walk_rejections;
        self.spell_rejections += other.spell_rejections;
        self.suppress_paired_snapback =
            self.suppress_paired_snapback || other.suppress_paired_snapback;
        self.suppress_paired_spell_reject =
            self.suppress_paired_spell_reject || other.suppress_paired_spell_reject;
        Ok(())
    }

    pub fn walk_summary(&self) -> HistSummary {
        let mut s = HistSummary::from_hist(&self.walk);
        s.rejections = self.walk_rejections;
        s
    }

    pub fn spell_summary(&self) -> HistSummary {
        let mut s = HistSummary::from_hist(&self.spell);
        s.rejections = self.spell_rejections;
        s
    }
}

fn record(h: &mut Histogram<u64>, d: Duration) {
    let us = d.as_micros().min(u128::from(u64::MAX)) as u64;
    let _ = h.record(us.max(1));
}

#[derive(Debug, Clone, Copy)]
pub struct HistSummary {
    pub samples: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub rejections: u64,
}

impl HistSummary {
    fn from_hist(h: &Histogram<u64>) -> Self {
        let samples = h.len();
        if samples == 0 {
            return Self {
                samples: 0,
                p50_us: 0,
                p95_us: 0,
                p99_us: 0,
                rejections: 0,
            };
        }
        Self {
            samples,
            p50_us: h.value_at_quantile(0.50),
            p95_us: h.value_at_quantile(0.95),
            p99_us: h.value_at_quantile(0.99),
            rejections: 0,
        }
    }

    fn json_object(self) -> String {
        format!(
            "{{\"samples\":{},\"p50_us\":{},\"p95_us\":{},\"p99_us\":{},\"rejections\":{}}}",
            self.samples, self.p50_us, self.p95_us, self.p99_us, self.rejections
        )
    }
}

#[derive(Debug, Clone)]
pub struct RunReport {
    pub bots: usize,
    pub duration_s: u64,
    pub warmup_s: u64,
    pub walk: HistSummary,
    pub spell_rune: HistSummary,
    pub bytes_in: u64,
    /// Server frames decrypted (one per server `write`); `bytes_in / frames_in` = payload per frame.
    pub frames_in: u64,
    pub bytes_out: u64,
    pub outstanding_at_end: u64,
    pub sends: u64,
    pub magic_effects: u64,
    pub animated_texts: u64,
    pub damage_sum: u64,
    pub damage_samples: u64,
    pub distance_shoots: u64,
    pub creature_health: u64,
    pub other_creature_moves: u64,
    pub unique_creatures: u64,
    pub bytes_discarded: u64,
    pub skip_failures: u64,
    pub unknown_opcodes: u64,
    pub unknown_opcode_first: Option<u8>,
    pub skip_failure_opcodes: HashMap<u8, u64>,
    pub unknown_opcode_counts: HashMap<u8, u64>,
    pub text_reject_counts: HashMap<String, u64>,
    pub skip_failure_first_peek: Option<String>,
    pub skip_failure_player_z: Option<u8>,
    pub unknown_opcode_peek: Option<String>,
    pub unknown_opcode_prev: Option<u8>,
    pub unknown_opcode_player_z: Option<u8>,
    /// Game sessions that dropped before the measurement window ended.
    pub disconnects: u64,
    /// Always 0: loadgen does not auto-reconnect during a run.
    pub reconnects: u64,
}

fn unknown_opcode_json(first: Option<u8>) -> String {
    match first {
        Some(op) => op.to_string(),
        None => "null".to_string(),
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn u8_histogram_json(map: &HashMap<u8, u64>) -> String {
    if map.is_empty() {
        return "{}".into();
    }
    let mut keys: Vec<u8> = map.keys().copied().collect();
    keys.sort_unstable();
    let parts: Vec<String> = keys.iter().map(|k| format!("\"{k}\":{}", map[k])).collect();
    format!("{{{}}}", parts.join(","))
}

fn str_histogram_json(map: &HashMap<String, u64>) -> String {
    if map.is_empty() {
        return "{}".into();
    }
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    let parts: Vec<String> = keys
        .iter()
        .map(|k| format!("\"{}\":{}", json_escape(k), map[*k]))
        .collect();
    format!("{{{}}}", parts.join(","))
}

impl RunReport {
    pub fn to_json(&self) -> String {
        format!(
            "{{\n  \"bots\": {},\n  \"duration_s\": {},\n  \"warmup_s\": {},\n  \"walk\": {},\n  \"spell_rune\": {},\n  \"bytes_in\": {},\n  \"frames_in\": {},\n  \"bytes_out\": {},\n  \"outstanding_at_end\": {},\n  \"sends\": {},\n  \"magic_effects\": {},\n  \"animated_texts\": {},\n  \"damage_sum\": {},\n  \"damage_samples\": {},\n  \"distance_shoots\": {},\n  \"creature_health\": {},\n  \"other_creature_moves\": {},\n  \"unique_creatures\": {},\n  \"bytes_discarded\": {},\n  \"skip_failures\": {},\n  \"unknown_opcodes\": {},\n  \"unknown_opcode_first\": {},\n  \"skip_failure_opcodes\": {},\n  \"unknown_opcode_counts\": {},\n  \"text_reject_counts\": {},\n  \"skip_failure_first_peek\": {},\n  \"skip_failure_player_z\": {},\n  \"unknown_opcode_peek\": {},\n  \"unknown_opcode_prev\": {},\n  \"unknown_opcode_player_z\": {},\n  \"disconnects\": {},\n  \"reconnects\": {}\n}}\n",
            self.bots,
            self.duration_s,
            self.warmup_s,
            self.walk.json_object(),
            self.spell_rune.json_object(),
            self.bytes_in,
            self.frames_in,
            self.bytes_out,
            self.outstanding_at_end,
            self.sends,
            self.magic_effects,
            self.animated_texts,
            self.damage_sum,
            self.damage_samples,
            self.distance_shoots,
            self.creature_health,
            self.other_creature_moves,
            self.unique_creatures,
            self.bytes_discarded,
            self.skip_failures,
            self.unknown_opcodes,
            unknown_opcode_json(self.unknown_opcode_first),
            u8_histogram_json(&self.skip_failure_opcodes),
            u8_histogram_json(&self.unknown_opcode_counts),
            str_histogram_json(&self.text_reject_counts),
            match &self.skip_failure_first_peek {
                Some(s) => format!("\"{}\"", json_escape(s)),
                None => "null".into(),
            },
            match self.skip_failure_player_z {
                Some(z) => z.to_string(),
                None => "null".into(),
            },
            match &self.unknown_opcode_peek {
                Some(s) => format!("\"{}\"", json_escape(s)),
                None => "null".into(),
            },
            unknown_opcode_json(self.unknown_opcode_prev),
            match self.unknown_opcode_player_z {
                Some(z) => z.to_string(),
                None => "null".into(),
            },
            self.disconnects,
            self.reconnects
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tfs_rust_common::Position;

    #[test]
    fn walk_ack_records_intended_delta() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        set.on_send(Correlate::Walk, t0, None, None);
        set.on_walk_ack(t0 + Duration::from_millis(12));
        let s = set.walk_summary();
        assert_eq!(s.samples, 1);
        assert!(s.p50_us >= 10_000);
    }

    #[test]
    fn spell_correlates_by_tile() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        let tile = Position::new(3, 4, 7);
        set.on_send(Correlate::SpellRune, t0, Some(tile), None);
        set.on_magic_effect(t0 + Duration::from_millis(5), tile, 11);
        assert_eq!(set.spell_summary().samples, 1);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn spell_effect_mismatch_does_not_ack() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        let tile = Position::new(3, 4, 7);
        // GFB expects FIREAREA (7); a blood splash (2) on the same tile is not
        // the cast landing.
        set.on_send(Correlate::SpellRune, t0, Some(tile), Some(7));
        set.on_magic_effect(t0 + Duration::from_millis(5), tile, 2);
        assert_eq!(set.spell_summary().samples, 0);
        assert_eq!(set.outstanding_count(), 1);
        set.on_magic_effect(t0 + Duration::from_millis(6), tile, 7);
        assert_eq!(set.spell_summary().samples, 1);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn spell_reject_retires_oldest_without_sample() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        let tile = Position::new(3, 4, 7);
        set.on_send(Correlate::SpellRune, t0, Some(tile), Some(7));
        set.on_spell_reject();
        let s = set.spell_summary();
        assert_eq!(s.samples, 0);
        assert_eq!(s.rejections, 1);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn spell_reject_with_empty_queue_is_noop() {
        let mut set = LatencySet::new().expect("hist");
        set.on_spell_reject();
        assert_eq!(set.spell_summary().rejections, 0);
    }

    #[test]
    fn second_spell_send_drops_unacked_head() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        let tile = Position::new(3, 4, 7);
        set.on_send(Correlate::SpellRune, t0, Some(tile), None);
        set.on_send(
            Correlate::SpellRune,
            t0 + Duration::from_millis(2500),
            Some(tile),
            None,
        );
        set.on_magic_effect(t0 + Duration::from_millis(2546), tile, 11);
        let s = set.spell_summary();
        assert_eq!(s.samples, 1);
        assert_eq!(s.rejections, 0);
        assert!(s.p50_us >= 40_000);
        assert!(s.p50_us < 200_000);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn spell_reject_after_supersede_is_suppressed() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        let tile = Position::new(3, 4, 7);
        set.on_send(Correlate::SpellRune, t0, Some(tile), None);
        set.on_send(
            Correlate::SpellRune,
            t0 + Duration::from_millis(2500),
            Some(tile),
            None,
        );
        set.on_spell_reject();
        set.on_magic_effect(t0 + Duration::from_millis(2546), tile, 11);
        let s = set.spell_summary();
        assert_eq!(s.samples, 1);
        assert_eq!(s.rejections, 0);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn spell_reject_without_supersede_still_rejects() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        let tile = Position::new(3, 4, 7);
        set.on_send(Correlate::SpellRune, t0, Some(tile), None);
        set.on_spell_reject();
        assert_eq!(set.spell_summary().rejections, 1);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn cancel_then_ack_one_sample_one_rejection() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        set.on_send(Correlate::Walk, t0, None, None);
        set.on_walk_cancel();
        set.on_send(Correlate::Walk, t0 + Duration::from_millis(200), None, None);
        set.on_walk_ack(t0 + Duration::from_millis(212));
        let s = set.walk_summary();
        assert_eq!(s.samples, 1);
        assert_eq!(s.rejections, 1);
        assert!(s.p50_us >= 10_000);
        assert!(s.p50_us < 200_000);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn cancel_with_empty_queue_is_noop() {
        let mut set = LatencySet::new().expect("hist");
        set.on_walk_cancel();
        assert_eq!(set.walk_summary().rejections, 0);
    }

    #[test]
    fn second_walk_send_drops_unacked_head() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        set.on_send(Correlate::Walk, t0, None, None);
        set.on_send(Correlate::Walk, t0 + Duration::from_millis(500), None, None);
        set.on_walk_cancel();
        set.on_walk_ack(t0 + Duration::from_millis(700));
        let s = set.walk_summary();
        assert_eq!(s.samples, 1);
        assert_eq!(s.rejections, 0);
        assert!(s.p50_us >= 100_000);
        assert!(s.p50_us < 500_000);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn snapback_without_supersede_still_rejects() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        set.on_send(Correlate::Walk, t0, None, None);
        set.on_walk_cancel();
        assert_eq!(set.walk_summary().rejections, 1);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn walk_text_reject_then_cancel_is_one_rejection() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        set.on_send(Correlate::Walk, t0, None, None);
        set.on_walk_text_reject();
        set.on_walk_cancel();
        let s = set.walk_summary();
        assert_eq!(s.samples, 0);
        assert_eq!(s.rejections, 1);
        assert_eq!(set.outstanding_count(), 0);
    }

    #[test]
    fn walk_text_reject_does_not_clear_on_empty_send() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        set.on_send(Correlate::Walk, t0, None, None);
        set.on_walk_text_reject();
        set.on_send(Correlate::Walk, t0 + Duration::from_millis(500), None, None);
        set.on_walk_cancel();
        let s = set.walk_summary();
        assert_eq!(s.samples, 0);
        assert_eq!(s.rejections, 1);
        assert_eq!(set.outstanding_count(), 1);
    }

    #[test]
    fn walk_json_includes_rejections() {
        let mut set = LatencySet::new().expect("hist");
        set.on_send(Correlate::Walk, Instant::now(), None, None);
        set.on_walk_cancel();
        let json = set.walk_summary().json_object();
        assert!(json.contains("\"rejections\":1"), "{json}");
        assert!(json.contains("\"samples\":0"), "{json}");
    }

    #[test]
    fn report_json_includes_unknown_opcode_fields() {
        let report = RunReport {
            bots: 1,
            duration_s: 1,
            warmup_s: 0,
            walk: HistSummary {
                samples: 0,
                p50_us: 0,
                p95_us: 0,
                p99_us: 0,
                rejections: 0,
            },
            spell_rune: HistSummary {
                samples: 0,
                p50_us: 0,
                p95_us: 0,
                p99_us: 0,
                rejections: 0,
            },
            bytes_in: 0,
            frames_in: 0,
            bytes_out: 0,
            outstanding_at_end: 0,
            sends: 0,
            magic_effects: 0,
            animated_texts: 0,
            damage_sum: 0,
            damage_samples: 0,
            distance_shoots: 0,
            creature_health: 0,
            other_creature_moves: 0,
            unique_creatures: 0,
            bytes_discarded: 0,
            skip_failures: 0,
            unknown_opcodes: 2,
            unknown_opcode_first: Some(0x15),
            skip_failure_opcodes: HashMap::from([(0x6A, 3)]),
            unknown_opcode_counts: HashMap::from([(0x15, 2)]),
            text_reject_counts: HashMap::from([("You are exhausted.".into(), 4)]),
            skip_failure_first_peek: Some("6301abcd".into()),
            skip_failure_player_z: Some(7),
            unknown_opcode_peek: Some("15aabb".into()),
            unknown_opcode_prev: Some(0x64),
            unknown_opcode_player_z: Some(8),
            disconnects: 1,
            reconnects: 0,
        };
        let json = report.to_json();
        assert!(json.contains("\"unknown_opcodes\": 2"), "{json}");
        assert!(json.contains("\"unknown_opcode_first\": 21"), "{json}");
        assert!(
            json.contains("\"skip_failure_opcodes\": {\"106\":3}"),
            "{json}"
        );
        assert!(
            json.contains("\"unknown_opcode_counts\": {\"21\":2}"),
            "{json}"
        );
        assert!(
            json.contains("\"text_reject_counts\": {\"You are exhausted.\":4}"),
            "{json}"
        );
        assert!(
            json.contains("\"skip_failure_first_peek\": \"6301abcd\""),
            "{json}"
        );
        assert!(json.contains("\"skip_failure_player_z\": 7"), "{json}");
        assert!(
            json.contains("\"unknown_opcode_peek\": \"15aabb\""),
            "{json}"
        );
        assert!(json.contains("\"unknown_opcode_prev\": 100"), "{json}");
        assert!(json.contains("\"unknown_opcode_player_z\": 8"), "{json}");
        assert!(json.contains("\"rejections\":0"), "{json}");
        assert!(json.contains("\"disconnects\": 1"), "{json}");
        assert!(json.contains("\"reconnects\": 0"), "{json}");
    }
}
