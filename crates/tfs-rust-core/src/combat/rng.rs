//! RNG helpers matching TFS `uniform_random` / `normal_random` / `triangular_random`.
// C++ reference: `tools.cpp`.

use rand::Rng;

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

/// TFS uses uniform distribution for “normal” melee rolls (real Tibia behavior).
#[inline]
pub fn normal_random<R: Rng + ?Sized>(rng: &mut R, min_n: i32, max_n: i32) -> i32 {
    uniform_random(rng, min_n, max_n)
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
