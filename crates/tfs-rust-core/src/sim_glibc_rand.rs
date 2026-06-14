//! glibc `rand()` / `srand()` for headless sim parity with C++ chase harness.
//!
//! C++ reference: `chase_kite_scenario.cc` `srand(TFS_SIM_SEED)`; `utils.cc` `random`;
//! `crskill.cc` `TSkillProbe::ProbeValue`; `crcombat.cc` `GetArmorStrength`.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rand::RngCore;
use tfs_rust_common::enums::Direction;

static SIM_GLIBC_RNG: AtomicBool = AtomicBool::new(false);
static SIM_RNG_TRACE: AtomicBool = AtomicBool::new(false);
static SIM_RNG_CALLS: AtomicU64 = AtomicU64::new(0);

/// One-time enable from [`crate::game_world::GameWorld::init_sim_rng_from_env`].
pub fn enable_sim_glibc_rng() {
    SIM_GLIBC_RNG.store(true, Ordering::Relaxed);
    let trace = std::env::var("TFS_SIM_RNG_TRACE")
        .is_ok_and(|v| !v.is_empty() && v != "0");
    SIM_RNG_TRACE.store(trace, Ordering::Relaxed);
    SIM_RNG_CALLS.store(0, Ordering::Relaxed);
}

pub fn sim_glibc_rng_enabled() -> bool {
    SIM_GLIBC_RNG.load(Ordering::Relaxed)
}

pub fn sim_rng_trace_enabled() -> bool {
    SIM_RNG_TRACE.load(Ordering::Relaxed)
}

pub fn sim_rng_call_count() -> u64 {
    SIM_RNG_CALLS.load(Ordering::Relaxed)
}

pub fn reset_sim_rng_call_count() {
    SIM_RNG_CALLS.store(0, Ordering::Relaxed);
}

fn draw_rand() -> i32 {
    // SAFETY: chase harness only; mirrors C++ `srand`/`rand` in `chase_kite_scenario.cc`.
    let value = unsafe { libc::rand() };
    let calls = SIM_RNG_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    if sim_rng_trace_enabled() {
        crate::chase_debug::log_rng_trace(calls, value);
    }
    value
}

/// C++ `utils.cc` `random(Min, Max)` — inclusive range via `rand() % Range`.
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

/// `rand() % modulus` when sim glibc mode is active.
pub fn sim_rand_mod(modulus: u32) -> u32 {
    debug_assert!(modulus > 0);
    if !sim_glibc_rng_enabled() {
        return 0;
    }
    (draw_rand() as u32) % modulus
}

/// Inclusive random — production uses `thread_rng`, sim uses glibc `random()`.
pub fn parity_random(min: i32, max: i32) -> i32 {
    if sim_glibc_rng_enabled() {
        sim_random(min, max)
    } else {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    }
}

/// Modulo roll — production uses `thread_rng`, sim uses glibc `rand()`.
pub fn parity_rand_mod(modulus: u32) -> u32 {
    debug_assert!(modulus > 0);
    if sim_glibc_rng_enabled() {
        sim_rand_mod(modulus)
    } else {
        use rand::Rng;
        rand::thread_rng().gen_range(0..modulus)
    }
}

/// C++ `TSkillProbe::ProbeValue` random factor — `((rand()%100)+(rand()%100))/2`.
pub fn sim_probe_random_factor() -> i32 {
    (sim_rand_mod(100) as i32 + sim_rand_mod(100) as i32) / 2
}

/// C++ dance sidestep order — `crnonpl.cc:2814-2819` (`rand()%5` → W,E,N,S,hold).
pub const DANCE_DIR_ORDER: [Option<Direction>; 5] = [
    Some(Direction::West),
    Some(Direction::East),
    Some(Direction::North),
    Some(Direction::South),
    None,
];

/// glibc-backed RNG for combat/idle parity when `TFS_SIM_SEED` is set.
#[derive(Debug, Clone, Copy, Default)]
pub struct SimGlibcRng;

impl RngCore for SimGlibcRng {
    fn next_u32(&mut self) -> u32 {
        if sim_glibc_rng_enabled() {
            draw_rand() as u32
        } else {
            0
        }
    }

    fn next_u64(&mut self) -> u64 {
        (self.next_u32() as u64) << 32 | self.next_u32() as u64
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for byte in dest.iter_mut() {
            *byte = self.next_u32() as u8;
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

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
}
