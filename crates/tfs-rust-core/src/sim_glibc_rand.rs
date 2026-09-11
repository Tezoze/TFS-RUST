//! Per-world glibc TYPE_3 `rand()` stream — live combat/AI and headless sim share one generator.
//!
//! C++ reference: `utils.cc` `random`; `crskill.cc` `TSkillProbe::ProbeValue`;
//! `crcombat.cc` `GetArmorStrength`; `chase_kite_scenario.cc` `srand(TFS_SIM_SEED)` (seeded by the
//! harness via [`crate::game_world::GameWorld::seed_parity_rng`], never from env inside this module).

use std::cell::Cell;

use tfs_rust_common::enums::Direction;

/// Per-world glibc TYPE_3 `rand()` state — isolates parallel tests (audit Finding 8/15).
/// Always compiled: used as `GameWorld::parity_rng` for the 772 beat-driven loop.
#[derive(Debug, Clone)]
pub struct GlibcRngState {
    next: Cell<u32>,
}

impl Default for GlibcRngState {
    fn default() -> Self {
        Self::seed(1)
    }
}

impl GlibcRngState {
    /// Mirrors `libc::srand(seed)` — glibc TYPE_3 initial state.
    pub fn seed(seed: u32) -> Self {
        Self {
            next: Cell::new(seed),
        }
    }

    /// One glibc `rand()` draw — TYPE_3: `(next/65536) % 32768`.
    pub fn rand(&self) -> i32 {
        let n = self
            .next
            .get()
            .wrapping_mul(1_103_515_245)
            .wrapping_add(12_345);
        self.next.set(n);
        ((n / 65_536) % 32_768) as i32
    }

    pub fn random(&self, min: i32, max: i32) -> i32 {
        let range = max - min + 1;
        if range <= 0 {
            return min;
        }
        min + (self.rand() % range)
    }

    pub fn rand_mod(&self, modulus: u32) -> u32 {
        debug_assert!(modulus > 0);
        let m = modulus as i32;
        (self.rand() % m) as u32
    }

    /// `ProbeValue` factor — `((rand()%M)+(rand()%M))/2` with `M = random_max+1`
    /// (`crskill.cc:543`; `random_max=99` → `% 100`).
    pub fn probe_random_factor(&self, random_max: i32) -> i32 {
        let m = (random_max.max(0) + 1) as u32;
        let a = self.rand_mod(m) as i32;
        let b = self.rand_mod(m) as i32;
        (a + b) / 2
    }

    /// Armor extra term — `rand() % (Armor/2)` (`crcombat.cc:304`).
    pub fn armor_rand_extra(&self, half: i32) -> i32 {
        self.rand_mod(half.max(1) as u32) as i32
    }

    /// Forward Fisher-Yates shuffle matching C++ `RandomShuffle`.
    pub fn random_shuffle<T>(&self, buf: &mut [T]) {
        let size = buf.len();
        if size < 2 {
            return;
        }
        let max = (size - 1) as i32;
        for min in 0..max {
            let swap = self.random(min, max) as usize;
            if swap != min as usize {
                buf.swap(min as usize, swap);
            }
        }
    }
}

/// Attribute the next glibc draw(s) to `site`. No-op until Phase 2 `target = "chase"` tracing.
pub struct SimRngTraceSiteGuard;

pub fn sim_rng_trace_site(site: &'static str) -> SimRngTraceSiteGuard {
    let _ = site;
    SimRngTraceSiteGuard
}

/// C++ dance sidestep order — `crnonpl.cc:2814-2819` (`rand()%5` → W,E,N,S,hold).
/// Always compiled: used by monster AI dance step selection.
pub const DANCE_DIR_ORDER: [Option<Direction>; 5] = [
    Some(Direction::West),
    Some(Direction::East),
    Some(Direction::North),
    Some(Direction::South),
    None,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dance_dir_order_n_s_matches_cpp_dest_y() {
        use tfs_rust_common::Position;
        let pos = Position::new(32361, 32290, 7);
        // C++ `crnonpl.cc:2817-2818` — case 2 `DestY-=1` (North), case 3 `DestY+=1` (South).
        assert_eq!(
            pos.offset(DANCE_DIR_ORDER[2].unwrap()),
            Position::new(32361, 32289, 7)
        );
        assert_eq!(
            pos.offset(DANCE_DIR_ORDER[3].unwrap()),
            Position::new(32361, 32291, 7)
        );
    }

    #[test]
    fn type3_rand_mod_is_stable_for_seed_772() {
        // ANSI TYPE_3 LCG — not host `libc::rand()` (glibc `random()` uses a state array).
        let rng = GlibcRngState::seed(772);
        assert_eq!(rng.rand_mod(5), 4);
        assert_eq!(rng.rand_mod(5), 3);
        assert_eq!(rng.rand_mod(5), 3);
    }

    #[test]
    fn random_stays_in_range() {
        let rng = GlibcRngState::seed(772);
        for _ in 0..8 {
            let v = rng.random(0, 99);
            assert!((0..=99).contains(&v));
        }
    }

    #[test]
    fn random_shuffle_is_permutation() {
        let rng = GlibcRngState::seed(772);
        let mut a = [0u8, 1, 2, 3, 4, 5, 6, 7];
        rng.random_shuffle(&mut a);
        let mut sorted = a;
        sorted.sort();
        assert_eq!(
            sorted,
            [0, 1, 2, 3, 4, 5, 6, 7],
            "forward Fisher-Yates must produce a permutation"
        );

        let mut one = [9u8];
        rng.random_shuffle(&mut one);
        assert_eq!(one, [9]);
        let mut empty: [u8; 0] = [];
        rng.random_shuffle(&mut empty);
    }
}
