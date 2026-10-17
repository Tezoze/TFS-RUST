//! Exhaust clocks. A spell, a weapon swing, and a two-object use each have their own wait.
//!
//! A Walk uses `EarliestWalkTime` and is not an exhaust.
//!
//! - Spell gate: `CastSpell` — `magic.cc:3399`. Spell wait moves forward in `CheckMana` — `magic.cc:770`.
//! - Weapon swing: `CalculateDelay` `TDAttack` waits for the later of both clocks — `cract.cc:984`.
//!   The attack clock moves forward in `TCombat::DelayAttack` — `crcombat.cc:524`.
//! - Two-object use: `CalculateDelay` `TDUse` when `Obj2 != 0` — `cract.cc:975`.

/// Move a deadline forward. A shorter delay does not pull an existing wait back.
pub(crate) fn push_deadline(current: u64, server_ms: u64, delay_ms: u64) -> u64 {
    current.max(server_ms.saturating_add(delay_ms))
}

/// A spell may start when the spell wait is over. The attack clock is not consulted.
pub(crate) fn spell_ready(earliest_spell_ms: u64, server_ms: u64) -> bool {
    server_ms >= earliest_spell_ms
}

/// Milliseconds until a weapon swing may start: the later of the attack wait and the spell wait.
pub(crate) fn swing_delay_ms(
    earliest_attack_ms: u64,
    earliest_spell_ms: u64,
    server_ms: u64,
) -> u64 {
    earliest_attack_ms
        .max(earliest_spell_ms)
        .saturating_sub(server_ms)
}

/// A weapon swing may start when [`swing_delay_ms`] is zero.
pub(crate) fn swing_ready(earliest_attack_ms: u64, earliest_spell_ms: u64, server_ms: u64) -> bool {
    swing_delay_ms(earliest_attack_ms, earliest_spell_ms, server_ms) == 0
}

/// Two-object use delay. A use with one object is not gated.
pub(crate) fn multiuse_delay_ms(earliest_multiuse_ms: u64, server_ms: u64, has_obj2: bool) -> u64 {
    if !has_obj2 {
        return 0;
    }
    earliest_multiuse_ms.saturating_sub(server_ms)
}

/// A two-object use may start when its own wait is over.
pub(crate) fn multiuse_ready(earliest_multiuse_ms: u64, server_ms: u64) -> bool {
    server_ms >= earliest_multiuse_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_ignores_the_attack_clock() {
        assert!(spell_ready(1000, 1000));
        assert!(!spell_ready(1500, 1000));
    }

    #[test]
    fn swing_waits_for_the_later_clock() {
        assert_eq!(swing_delay_ms(1000, 2500, 1000), 1500);
        assert_eq!(swing_delay_ms(2500, 1000, 1000), 1500);
        assert!(swing_ready(1000, 1000, 1000));
        assert!(!swing_ready(1000, 1001, 1000));
    }

    #[test]
    fn multiuse_is_a_third_clock_and_single_use_is_open() {
        assert_eq!(multiuse_delay_ms(5000, 1000, false), 0);
        assert_eq!(multiuse_delay_ms(5000, 1000, true), 4000);
        assert!(!multiuse_ready(5000, 1000));
        assert!(multiuse_ready(1000, 1000));
    }

    #[test]
    fn a_shorter_delay_does_not_pull_the_wait_back() {
        assert_eq!(push_deadline(5_000, 1_000, 100), 5_000);
        assert_eq!(push_deadline(5_000, 1_000, 4_500), 5_500);
    }
}
