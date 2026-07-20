//! RNG helpers matching TFS `uniform_random` / `normal_random` / `triangular_random`.
// C++ reference: `tools.cpp`.
//
// Production combat/AI uses [`crate::sim_glibc_rand::GlibcRngState`] via the `*_glibc`
// helpers below. The generic `Rng` variants remain for non-simulation call sites
// (e.g. walk UI) that still use `thread_rng`.

use rand::Rng;

use crate::sim_glibc_rand::GlibcRngState;

#[inline]
pub fn uniform_random<R: Rng + ?Sized>(rng: &mut R, min_n: i32, max_n: i32) -> i32 {
    if min_n == max_n {
        return min_n;
    }
    let (lo, hi) = if min_n <= max_n {
        (min_n, max_n)
    } else {
        (max_n, min_n)
    };
    rng.gen_range(lo..=hi)
}

/// Inclusive uniform on the per-world glibc stream (sim harness overrides when enabled).
#[inline]
pub fn uniform_random_glibc(parity: &GlibcRngState, min_n: i32, max_n: i32) -> i32 {
    if min_n == max_n {
        return min_n;
    }
    let (lo, hi) = if min_n <= max_n {
        (min_n, max_n)
    } else {
        (max_n, min_n)
    };
    #[cfg(any(test, feature = "sim"))]
    if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
        return crate::sim_glibc_rand::sim_random(lo, hi);
    }
    parity.random(lo, hi)
}

/// TFS uses uniform distribution for “normal” melee rolls (real Tibia behavior).
#[inline]
pub fn normal_random<R: Rng + ?Sized>(rng: &mut R, min_n: i32, max_n: i32) -> i32 {
    uniform_random(rng, min_n, max_n)
}

#[inline]
pub fn normal_random_glibc(parity: &GlibcRngState, min_n: i32, max_n: i32) -> i32 {
    uniform_random_glibc(parity, min_n, max_n)
}

/// Average of two independent uniform rolls (slight bell curve).
#[inline]
pub fn triangular_random<R: Rng + ?Sized>(rng: &mut R, min_n: i32, max_n: i32) -> i32 {
    if min_n == max_n {
        return min_n;
    }
    let r1 = normal_random(rng, min_n, max_n);
    let r2 = normal_random(rng, min_n, max_n);
    (r1 + r2) / 2
}

#[inline]
pub fn triangular_random_glibc(parity: &GlibcRngState, min_n: i32, max_n: i32) -> i32 {
    if min_n == max_n {
        return min_n;
    }
    let r1 = normal_random_glibc(parity, min_n, max_n);
    let r2 = normal_random_glibc(parity, min_n, max_n);
    (r1 + r2) / 2
}
