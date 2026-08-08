//! glibc `rand()` / `srand()` for headless sim parity with C++ chase harness.
//!
//! C++ reference: `chase_kite_scenario.cc` `srand(TFS_SIM_SEED)`; `utils.cc` `random`;
//! `crskill.cc` `TSkillProbe::ProbeValue`; `crcombat.cc` `GetArmorStrength`.
//!
//! Phase 2: the glibc parity stream, trace infrastructure, and `SimGlibcRng` are
//! `#[cfg(any(test, feature = "sim"))]` — excluded from production builds. The
//! per-world `GlibcRngState`, `DANCE_DIR_ORDER`, and `parity_*` dispatchers are always
//! compiled (they serve the 772 beat-driven loop and fall back to `thread_rng` when
//! sim mode is inactive).

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

// ---------------------------------------------------------------------------
// Sim-only state + trace infrastructure.
// ---------------------------------------------------------------------------

#[cfg(any(test, feature = "sim"))]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[cfg(any(test, feature = "sim"))]
static SIM_GLIBC_RNG: AtomicBool = AtomicBool::new(false);
#[cfg(any(test, feature = "sim"))]
static SIM_RNG_TRACE: AtomicBool = AtomicBool::new(false);
#[cfg(any(test, feature = "sim"))]
static SIM_RNG_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(any(test, feature = "sim"))]
static HARNESS_MELEE_REALIGN_DONE: AtomicBool = AtomicBool::new(false);

#[cfg(any(test, feature = "sim"))]
thread_local! {
    static RNG_TRACE_SITE: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// Attribute the next glibc draw(s) to `site` in [`TFS_SIM_RNG_TRACE`] output.
/// No-op guard in production builds (field is never set without the sim feature).
pub struct SimRngTraceSiteGuard {
    #[cfg(any(test, feature = "sim"))]
    prev: Option<&'static str>,
}

pub fn sim_rng_trace_site(site: &'static str) -> SimRngTraceSiteGuard {
    #[cfg(any(test, feature = "sim"))]
    {
        let prev = RNG_TRACE_SITE.with(|c| {
            let p = c.get();
            c.set(Some(site));
            p
        });
        SimRngTraceSiteGuard { prev }
    }
    #[cfg(not(any(test, feature = "sim")))]
    {
        let _ = site;
        SimRngTraceSiteGuard {}
    }
}

impl Drop for SimRngTraceSiteGuard {
    fn drop(&mut self) {
        #[cfg(any(test, feature = "sim"))]
        {
            RNG_TRACE_SITE.with(|c| c.set(self.prev));
        }
    }
}

// ---------------------------------------------------------------------------
// Sim mode flag + enable/init — only compiled for test/sim.
// ---------------------------------------------------------------------------

/// One-time enable from [`crate::game_world::GameWorld::init_sim_rng_from_env`].
#[cfg(any(test, feature = "sim"))]
pub fn enable_sim_glibc_rng() {
    SIM_GLIBC_RNG.store(true, Ordering::Relaxed);
    let trace = std::env::var("TFS_SIM_RNG_TRACE").is_ok_and(|v| !v.is_empty() && v != "0");
    SIM_RNG_TRACE.store(trace, Ordering::Relaxed);
    SIM_RNG_CALLS.store(0, Ordering::Relaxed);
    reset_harness_melee_realign_done();
}

/// Whether sim glibc RNG mode is active.
/// Always compiled: production code checks this to decide sim vs `thread_rng` path.
/// Returns `false` in production builds (no sim feature, not test mode).
#[cfg(any(test, feature = "sim"))]
pub fn sim_glibc_rng_enabled() -> bool {
    SIM_GLIBC_RNG.load(Ordering::Relaxed)
}

#[cfg(any(test, feature = "sim"))]
pub fn sim_rng_trace_enabled() -> bool {
    SIM_RNG_TRACE.load(Ordering::Relaxed)
}

#[cfg(any(test, feature = "sim"))]
pub fn sim_rng_call_count() -> u64 {
    SIM_RNG_CALLS.load(Ordering::Relaxed)
}

#[cfg(any(test, feature = "sim"))]
pub fn reset_sim_rng_call_count() {
    SIM_RNG_CALLS.store(0, Ordering::Relaxed);
}

#[cfg(any(test, feature = "sim"))]
pub fn harness_melee_realign_done() -> bool {
    HARNESS_MELEE_REALIGN_DONE.load(Ordering::Relaxed)
}

#[cfg(any(test, feature = "sim"))]
pub fn mark_harness_melee_realign_done() {
    HARNESS_MELEE_REALIGN_DONE.store(true, Ordering::Relaxed);
}

#[cfg(any(test, feature = "sim"))]
pub fn reset_harness_melee_realign_done() {
    HARNESS_MELEE_REALIGN_DONE.store(false, Ordering::Relaxed);
}

/// Re-seed glibc `rand()` from `TFS_SIM_SEED` — chase harness appear/combat parity.
#[cfg(any(test, feature = "sim"))]
pub fn resync_harness_glibc_rng_from_env() {
    if let Ok(seed_str) = std::env::var("TFS_SIM_SEED") {
        if let Ok(seed) = seed_str.parse::<u64>() {
            if sim_glibc_rng_enabled() {
                // SAFETY: harness-only; mirrors C++ `ResyncHarnessRng` in `chase_kite_scenario.cc`.
                unsafe { libc::srand(seed as u32) };
                reset_sim_rng_call_count();
                if sim_rng_trace_enabled() {
                    crate::chase_debug::log_rng_resync(seed);
                }
            }
        }
    }
}

#[cfg(any(test, feature = "sim"))]
fn draw_rand() -> i32 {
    // SAFETY: chase harness only; mirrors C++ `srand`/`rand` in `chase_kite_scenario.cc`.
    let value = unsafe { libc::rand() };
    let calls = SIM_RNG_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    if sim_rng_trace_enabled() {
        let site = RNG_TRACE_SITE.with(|c| c.get());
        crate::chase_debug::log_rng_trace(calls, value, site);
    }
    value
}

/// C++ `utils.cc` `random(Min, Max)` — inclusive range via `rand() % Range`.
#[cfg(any(test, feature = "sim"))]
pub fn sim_random(min: i32, max: i32) -> i32 {
    if !sim_glibc_rng_enabled() {
        return min;
    }
    let range = max - min + 1;
    if range <= 0 {
        return min;
    }
    min + (draw_rand() % range)
}

/// `rand() % modulus` when sim glibc mode is active — signed trunc toward zero (`crskill.cc`).
#[cfg(any(test, feature = "sim"))]
pub fn sim_rand_mod(modulus: u32) -> u32 {
    debug_assert!(modulus > 0);
    if !sim_glibc_rng_enabled() {
        return 0;
    }
    let m = modulus as i32;
    (draw_rand() % m) as u32
}

// ---------------------------------------------------------------------------
// Parity dispatchers — always compiled.
// In sim mode: route through glibc `rand()`. In production: use `thread_rng`.
// ---------------------------------------------------------------------------

/// Inclusive random — production uses `thread_rng`, sim uses glibc `random()`.
/// Prefer [`crate::game_world::GameWorld::parity_random`] for live simulation draws.
pub fn parity_random(min: i32, max: i32) -> i32 {
    #[cfg(any(test, feature = "sim"))]
    if sim_glibc_rng_enabled() {
        return sim_random(min, max);
    }
    use rand::RngExt;
    rand::rng().random_range(min..=max)
}

/// Modulo roll — production uses `thread_rng`, sim uses glibc `rand()`.
/// Prefer [`GameWorld::parity_rand_mod`] for live simulation draws.
#[allow(dead_code)]
pub fn parity_rand_mod(modulus: u32) -> u32 {
    debug_assert!(modulus > 0);
    #[cfg(any(test, feature = "sim"))]
    if sim_glibc_rng_enabled() {
        return sim_rand_mod(modulus);
    }
    use rand::RngExt;
    rand::rng().random_range(0..modulus)
}

/// C++ `RandomShuffle` (`common.hh:206`) — **forward** Fisher-Yates over glibc `random(Min, Size-1)`
/// (inclusive). Mirrors the C++ draw count and order exactly (one `random` call per index `0..Size-1`),
/// unlike the rand crate's backward `SliceRandom::shuffle`. Draws come from the parity stream (glibc
/// in sim mode, `thread_rng` live), so flee/spawn shuffles advance the same stream as C++.
#[allow(dead_code)]
pub fn parity_random_shuffle<T>(buf: &mut [T]) {
    let size = buf.len();
    if size < 2 {
        return;
    }
    let max = (size - 1) as i32;
    for min in 0..max {
        let swap = parity_random(min, max) as usize;
        if swap != min as usize {
            buf.swap(min as usize, swap);
        }
    }
}

/// C++ `TSkillProbe::ProbeValue` random factor — `((rand()%100)+(rand()%100))/2`.
#[cfg(any(test, feature = "sim"))]
pub fn sim_probe_random_factor() -> i32 {
    let _a = sim_rng_trace_site("probe_rand_a");
    let a = sim_rand_mod(100) as i32;
    let _b = sim_rng_trace_site("probe_rand_b");
    let b = sim_rand_mod(100) as i32;
    (a + b) / 2
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
    fn glibc_rand_matches_linux_seed_772() {
        unsafe { libc::srand(772) };
        enable_sim_glibc_rng();
        assert_eq!(sim_rand_mod(5), 2);
        assert_eq!(sim_rand_mod(5), 0);
        assert_eq!(sim_rand_mod(5), 0);
        SIM_GLIBC_RNG.store(false, Ordering::Relaxed);
    }

    #[test]
    fn sim_random_stays_in_range() {
        unsafe { libc::srand(772) };
        enable_sim_glibc_rng();
        for _ in 0..8 {
            let v = sim_random(0, 99);
            assert!((0..=99).contains(&v));
        }
        SIM_GLIBC_RNG.store(false, Ordering::Relaxed);
    }

    #[test]
    fn parity_random_shuffle_is_permutation() {
        unsafe { libc::srand(772) };
        enable_sim_glibc_rng();
        let mut a = [0u8, 1, 2, 3, 4, 5, 6, 7];
        parity_random_shuffle(&mut a);
        let mut sorted = a;
        sorted.sort();
        assert_eq!(
            sorted,
            [0, 1, 2, 3, 4, 5, 6, 7],
            "forward Fisher-Yates must produce a permutation"
        );

        // Trivial sizes are no-ops.
        let mut one = [9u8];
        parity_random_shuffle(&mut one);
        assert_eq!(one, [9]);
        let mut empty: [u8; 0] = [];
        parity_random_shuffle(&mut empty);
        SIM_GLIBC_RNG.store(false, Ordering::Relaxed);
    }
}
