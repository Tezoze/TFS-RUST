//! `hdrhistogram` walk-ack and spell/rune latency. Intended-time, not send-time.

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
    outstanding_spell: Vec<(Instant, tfs_rust_common::Position)>,
}

impl LatencySet {
    pub fn new() -> Result<Self> {
        Ok(Self {
            walk: Histogram::new(SIGFIGS).map_err(|e| anyhow!("walk histogram: {e}"))?,
            spell: Histogram::new(SIGFIGS).map_err(|e| anyhow!("spell histogram: {e}"))?,
            outstanding_walk: Vec::new(),
            outstanding_spell: Vec::new(),
        })
    }

    /// Record an intended send. FIFO per correlation type.
    pub fn on_send(
        &mut self,
        kind: Correlate,
        intended: Instant,
        tile: Option<tfs_rust_common::Position>,
    ) {
        match kind {
            Correlate::Walk => self.outstanding_walk.push(intended),
            Correlate::SpellRune => {
                if let Some(pos) = tile {
                    self.outstanding_spell.push((intended, pos));
                }
            }
        }
    }

    pub fn on_walk_ack(&mut self, now: Instant) {
        if let Some(intended) = self.outstanding_walk.first().copied() {
            self.outstanding_walk.remove(0);
            record(&mut self.walk, now.saturating_duration_since(intended));
        }
    }

    pub fn on_magic_effect(&mut self, now: Instant, pos: tfs_rust_common::Position) {
        if let Some(idx) = self.outstanding_spell.iter().position(|(_, p)| *p == pos) {
            let (intended, _) = self.outstanding_spell.remove(idx);
            record(&mut self.spell, now.saturating_duration_since(intended));
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
        Ok(())
    }

    pub fn walk_summary(&self) -> HistSummary {
        HistSummary::from_hist(&self.walk)
    }

    pub fn spell_summary(&self) -> HistSummary {
        HistSummary::from_hist(&self.spell)
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
            };
        }
        Self {
            samples,
            p50_us: h.value_at_quantile(0.50),
            p95_us: h.value_at_quantile(0.95),
            p99_us: h.value_at_quantile(0.99),
        }
    }

    fn json_object(self) -> String {
        format!(
            "{{\"samples\":{},\"p50_us\":{},\"p95_us\":{},\"p99_us\":{}}}",
            self.samples, self.p50_us, self.p95_us, self.p99_us
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
}

impl RunReport {
    pub fn to_json(&self) -> String {
        format!(
            "{{\n  \"bots\": {},\n  \"duration_s\": {},\n  \"warmup_s\": {},\n  \"walk\": {},\n  \"spell_rune\": {},\n  \"bytes_in\": {},\n  \"bytes_out\": {},\n  \"outstanding_at_end\": {},\n  \"sends\": {},\n  \"magic_effects\": {},\n  \"animated_texts\": {},\n  \"damage_sum\": {},\n  \"damage_samples\": {},\n  \"distance_shoots\": {},\n  \"creature_health\": {},\n  \"other_creature_moves\": {},\n  \"unique_creatures\": {}\n}}\n",
            self.bots,
            self.duration_s,
            self.warmup_s,
            self.walk.json_object(),
            self.spell_rune.json_object(),
            self.bytes_in,
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
            self.unique_creatures
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_common::Position;

    #[test]
    fn walk_ack_records_intended_delta() {
        let mut set = LatencySet::new().expect("hist");
        let t0 = Instant::now();
        set.on_send(Correlate::Walk, t0, None);
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
        set.on_send(Correlate::SpellRune, t0, Some(tile));
        set.on_magic_effect(t0 + Duration::from_millis(5), tile);
        assert_eq!(set.spell_summary().samples, 1);
        assert_eq!(set.outstanding_count(), 0);
    }
}
