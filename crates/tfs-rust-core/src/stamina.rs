//! Stamina pool for clients from protocol 780.
//!
//! C++ reference: `800src/player.h` `staminaMinutes = 3360`;
//! `800src/player.cpp` `Player::gainExperience` (a bar at 0 grants no experience);
//! `800src/luascript.cpp` `player:getStamina` / `player:setStamina`.
//!
//! That tree does not drain or regenerate. Rates come from
//! `data/formulas/<era>.lua` `stamina.regenSeconds` / `drainSeconds`.
//! Rest is "not in combat": `earliest_logout_round` has expired (swords icon).

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Column default in `schema.sql` (`42 * 60`). Raised to the formula max on login
/// when the formula max is a different size, so an 8.0 character shows 56:00.
const SCHEMA_DEFAULT_STAMINA: u16 = 2520;

/// One creature-counter fire (`subsystem_counters.rs` `RESET_MS`).
const TICK_MS: u64 = 1000;

/// Login adjustment: lift the old 42-hour default, clamp, then apply offline rest.
pub fn stamina_after_login(stored: u16, lastlogout: u64, max: u16, regen_seconds: u32) -> u16 {
    if max == 0 {
        return stored;
    }
    let minutes = if stored == SCHEMA_DEFAULT_STAMINA && max != SCHEMA_DEFAULT_STAMINA {
        max
    } else {
        stored.min(max)
    };
    if regen_seconds == 0 || lastlogout == 0 || minutes >= max {
        return minutes;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let offline = now.saturating_sub(lastlogout);
    let gained = offline / u64::from(regen_seconds);
    let gained = u16::try_from(gained).unwrap_or(u16::MAX);
    minutes.saturating_add(gained).min(max)
}

impl GameWorld {
    /// Rest regen and combat drain. Called once per creature-counter fire.
    pub(crate) fn tick_stamina(&mut self) {
        let max = self.mechanics.profile.stamina_max_minutes;
        if max == 0 {
            return;
        }
        let regen_ms = u64::from(self.mechanics.profile.stamina_regen_seconds).saturating_mul(1000);
        let drain_ms = u64::from(self.mechanics.profile.stamina_drain_seconds).saturating_mul(1000);
        let round_nr = self.round_nr;
        let mut changed: Vec<CreatureId> = Vec::new();

        for (cid, kind) in self.creatures.iter_mut() {
            let CreatureKind::Player(p) = kind else {
                continue;
            };
            if p.base.health <= 0 || p.base.is_dead {
                continue;
            }
            let hunting = p.earliest_logout_round > round_nr;
            if hunting {
                p.stamina_rest_ms = 0;
                if drain_ms == 0 || p.stamina_minutes == 0 {
                    continue;
                }
                p.stamina_hunt_ms = p.stamina_hunt_ms.saturating_add(TICK_MS);
                if p.stamina_hunt_ms >= drain_ms {
                    p.stamina_hunt_ms -= drain_ms;
                    p.stamina_minutes -= 1;
                    changed.push(cid);
                }
            } else {
                p.stamina_hunt_ms = 0;
                if regen_ms == 0 || p.stamina_minutes >= max {
                    p.stamina_rest_ms = 0;
                    continue;
                }
                p.stamina_rest_ms = p.stamina_rest_ms.saturating_add(TICK_MS);
                if p.stamina_rest_ms >= regen_ms {
                    p.stamina_rest_ms -= regen_ms;
                    p.stamina_minutes = p.stamina_minutes.saturating_add(1).min(max);
                    changed.push(cid);
                }
            }
        }

        for cid in changed {
            self.send_player_stats(cid);
        }
    }

    /// `player:setStamina` — clamp to the formula max (`800src/luascript.cpp`).
    pub(crate) fn player_set_stamina_u64(&mut self, player_u64: u64, minutes: u16) -> bool {
        let Some(cid) = self.resolve_creature_u64(player_u64) else {
            return false;
        };
        let max = self.mechanics.profile.stamina_max_minutes;
        let capped = if max == 0 { minutes } else { minutes.min(max) };
        {
            let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
                return false;
            };
            p.stamina_minutes = capped;
            p.stamina_rest_ms = 0;
            p.stamina_hunt_ms = 0;
        }
        self.send_player_stats(cid);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::test_support::{insert_player, minimal_world, test_player};
    use tfs_rust_common::Position;

    #[test]
    fn login_lifts_schema_default_and_keeps_empty() {
        assert_eq!(stamina_after_login(2520, 0, 56 * 60, 180), 56 * 60);
        assert_eq!(stamina_after_login(0, 0, 56 * 60, 180), 0);
        assert_eq!(stamina_after_login(100, 0, 56 * 60, 180), 100);
        assert_eq!(stamina_after_login(4000, 0, 56 * 60, 180), 56 * 60);
        assert_eq!(stamina_after_login(2520, 0, 0, 180), 2520);
    }

    #[test]
    fn login_offline_rest_uses_regen_seconds() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last = now.saturating_sub(180 * 2);
        assert_eq!(stamina_after_login(10, last, 56 * 60, 180), 12);
    }

    #[test]
    fn rest_tick_recovers_one_minute() {
        let mut world = minimal_world();
        world.mechanics.profile.stamina_regen_seconds = 1;
        let mut player = test_player("Rest", Position::new(100, 100, 7));
        player.stamina_minutes = 10;
        player.earliest_logout_round = 0;
        let cid = insert_player(&mut world, player);
        world.round_nr = 5;
        world.tick_stamina();
        let CreatureKind::Player(p) = world.creatures.get(cid).unwrap() else {
            panic!("player");
        };
        assert_eq!(p.stamina_minutes, 11);
    }

    #[test]
    fn combat_tick_spends_one_minute() {
        let mut world = minimal_world();
        world.mechanics.profile.stamina_drain_seconds = 1;
        let mut player = test_player("Hunt", Position::new(100, 100, 7));
        player.stamina_minutes = 10;
        player.earliest_logout_round = 50;
        let cid = insert_player(&mut world, player);
        world.round_nr = 5;
        world.tick_stamina();
        let CreatureKind::Player(p) = world.creatures.get(cid).unwrap() else {
            panic!("player");
        };
        assert_eq!(p.stamina_minutes, 9);
    }

    #[test]
    fn disabled_pool_does_not_move() {
        let mut world = minimal_world();
        world.mechanics.profile.stamina_max_minutes = 0;
        world.mechanics.profile.stamina_regen_seconds = 1;
        let mut player = test_player("Off", Position::new(100, 100, 7));
        player.stamina_minutes = 10;
        let cid = insert_player(&mut world, player);
        world.tick_stamina();
        let CreatureKind::Player(p) = world.creatures.get(cid).unwrap() else {
            panic!("player");
        };
        assert_eq!(p.stamina_minutes, 10);
    }
}
