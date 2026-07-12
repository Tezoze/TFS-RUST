//! PC-4 — Fight/chase/secure mode setters + PVP gating.
//!
//! C++ reference (mechanics, `tibia-game-master/src/`):
//! - `TCombat::SetAttackMode` — `crcombat.cc:325-337` (change → `DelayAttack(2000)`).
//! - `TCombat::SetChaseMode` — `crcombat.cc:339-346` (NONE/CLOSE only).
//! - `TCombat::SetSecureMode` — `crcombat.cc:348-355` (DISABLED/ENABLED only).
//! - `TCreature::BlockLogout` — `crmain.cc:433-453` (sets `EarliestLogoutRound` +
//!   `EarliestProtectionZoneRound`).
//! - `TPlayer::IsAttackJustified` — `crplayer.cc:1438-1460` (aggressor/party/attacker check).
//! - Secure-mode gate — `crcombat.cc:374-381` (`SetAttackDest` `!Follow`) + `:563-568` (`Attack`).
//!
//! Skull/frag subsystem (`RecordAttack`, aggressor flag, `AttackedPlayers` list, skull
//! broadcast, `RecordMurder`, playerkiller timer, banishment) is **deferred** to a dedicated
//! PvP phase (PC-4 scope decision: "defer all skulls"). `is_attack_justified` is stubbed to
//! `false` — secure mode blocks all player-vs-player attacks when `WorldType == Pvp` until the
//! full aggressor/party tracking lands.

use tfs_rust_common::WorldType;

use crate::combat::math::FightMode;
use crate::creature::{ChaseMode, CreatureKind};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::player_flags::{
    has_player_flag, PLAYER_FLAG_CANNOT_USE_COMBAT, PLAYER_FLAG_CANNOT_BE_ATTACKED,
};

impl GameWorld {
    /// 772 `0xA7` `FIGHT_MODES` packet body — `SetAttackMode` + `SetChaseMode` + `SetSecureMode`
    /// (`crcombat.cc:325-355`, `receiving.cc` `FIGHT_MODES`).
    ///
    /// - `SetAttackMode`: validates `OFFENSIVE`/`BALANCED`/`DEFENSIVE`; on change, calls
    ///   `DelayAttack(2000)` before writing `AttackMode` (`crcombat.cc:333-336`).
    /// - `SetChaseMode`: validates `NONE`/`CLOSE`; writes `ChaseMode` (`crcombat.cc:339-345`).
    ///   Does **not** override `Close` forced by an active follow (`Following ⇒ CLOSE`).
    /// - `SetSecureMode`: validates `DISABLED`/`ENABLED`; writes `SecureMode`
    ///   (`crcombat.cc:348-354`).
    pub(crate) fn player_set_fight_modes(
        &mut self,
        cid: CreatureId,
        raw_fight_mode: u8,
        raw_chase_mode: u8,
        raw_secure_mode: u8,
    ) {
        let server_ms = self.server_ms;

        // `SetAttackMode` — `crcombat.cc:325-337`.
        let new_attack_mode = FightMode::from_wire(raw_fight_mode);
        let attack_mode_changed = self.creatures.get(cid).is_some_and(|k| match k {
            CreatureKind::Player(p) => p.attack_mode != new_attack_mode,
            _ => false,
        });
        if attack_mode_changed {
            // `DelayAttack(2000)` before writing the new mode (`crcombat.cc:334`).
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 2000);
            }
        }

        // `SetChaseMode` — `crcombat.cc:339-346` (only NONE/CLOSE accepted).
        let chase = match raw_chase_mode {
            0 => ChaseMode::None,
            1 => ChaseMode::Close,
            other => {
                tracing::warn!(
                    conn_id = ?cid,
                    raw_chase_mode = other,
                    "FightModes: 772 SetChaseMode only accepts NONE(0)/CLOSE(1); clamping to NONE"
                );
                ChaseMode::None
            }
        };

        // `SetSecureMode` — `crcombat.cc:348-355` (only DISABLED/ENABLED accepted).
        let secure = match raw_secure_mode {
            0 => false,
            1 => true,
            other => {
                tracing::warn!(
                    conn_id = ?cid,
                    raw_secure_mode = other,
                    "FightModes: 772 SetSecureMode only accepts DISABLED(0)/ENABLED(1); clamping to DISABLED"
                );
                false
            }
        };

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.attack_mode = new_attack_mode;
            // Do not override `Close` forced by an active follow (`Following ⇒ CLOSE`).
            if p.base.follow_target.is_none() {
                p.base.chase_mode = chase;
            }
            p.secure_mode = secure;
        }
    }

    /// 772 `TCreature::BlockLogout` — `crmain.cc:433-453`.
    ///
    /// Sets `EarliestLogoutRound = max(., RoundNr + Delay)` and, when `block_pz` is true (or
    /// the player already has a pending PZ block), `EarliestProtectionZoneRound = max(.,
    /// RoundNr + Delay)`. In `NON_PVP` worlds, `block_pz` is cleared (`crmain.cc:434-436`).
    /// Skipped for non-players and for players with the `NO_LOGOUT_BLOCK` right (deferred —
    /// no group flag mapping yet, so all players are subject to the block).
    pub(crate) fn player_block_logout(&mut self, cid: CreatureId, delay_rounds: u32, block_pz: bool) {
        let world_type = self.pvp_config.world_type;
        let round_nr = self.round_nr;
        // `NON_PVP` clears `BlockProtectionZone` (`crmain.cc:434-436`).
        let block_pz = block_pz && world_type != WorldType::NoPvp;

        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return;
        };

        // `EarliestProtectionZoneRound` — only extended when `block_pz` or already pending
        // (`crmain.cc:439-443`).
        if block_pz || p.earliest_protection_zone_round > round_nr {
            let earliest = round_nr.saturating_add(delay_rounds);
            if p.earliest_protection_zone_round < earliest {
                p.earliest_protection_zone_round = earliest;
            }
        }

        // `EarliestLogoutRound` — always extended (`crmain.cc:450-453`).
        let earliest = round_nr.saturating_add(delay_rounds);
        if p.earliest_logout_round < earliest {
            p.earliest_logout_round = earliest;
        }
    }

    /// 772 `TPlayer::IsAttackJustified` — `crplayer.cc:1438-1460`.
    ///
    /// In the full system, returns `true` when the victim is an aggressor, in party with the
    /// attacker, or has attacked the attacker. **Stub**: returns `false` (no one is justified)
    /// — the aggressor/party/attacked-players tracking is deferred to the PvP skull phase.
    /// This means secure mode blocks **all** player-vs-player attacks when `WorldType == Pvp`
    /// until the full subsystem lands.
    pub(crate) fn player_is_attack_justified(&self, _attacker: CreatureId, _victim: CreatureId) -> bool {
        // TODO(pvp-phase): implement aggressor flag + AttackedPlayers list + party check.
        // `IsAttackJustified` returns `true` when WorldType != NORMAL (`crplayer.cc:1445`), but
        // the secure-mode gate only fires when `WorldType == NORMAL` (`crcombat.cc:564`), so the
        // `WorldType != Pvp` case is handled by the caller before reaching this stub.
        false
    }

    /// Secure-mode PVP gate — `crcombat.cc:374-381` (`SetAttackDest` `!Follow`) + `:563-568`
    /// (`Attack`). Returns `true` when the attack is blocked by secure mode.
    ///
    /// Fires when: attacker is a player, target is a player, `SecureMode == ENABLED`,
    /// `WorldType == Pvp`, and `!IsAttackJustified(target)`. In `PvpEnforced` / `NoPvp` worlds,
    /// secure mode does not gate (the `WorldType == NORMAL` check in C++ maps to `== Pvp`).
    pub(crate) fn player_secure_mode_blocks_attack(
        &self,
        attacker: CreatureId,
        target: CreatureId,
    ) -> bool {
        if self.pvp_config.world_type != WorldType::Pvp {
            return false;
        }
        let (attacker_secure, both_players) = match (
            self.creatures.get(attacker),
            self.creatures.get(target),
        ) {
            (Some(CreatureKind::Player(a)), Some(CreatureKind::Player(_))) => (a.secure_mode, true),
            _ => return false,
        };
        both_players && attacker_secure && !self.player_is_attack_justified(attacker, target)
    }

    /// `CheckRight(NO_ATTACK)` equivalent — `crcombat.cc:391-394,589-593`. Returns `true` when
    /// the player has the `CannotUseCombat` group flag (772 `NO_ATTACK` right), blocking all
    /// attack actions. Non-players are never blocked.
    pub(crate) fn player_attack_blocked_by_right(&self, cid: CreatureId) -> bool {
        let flags = self.player_group_flags(cid);
        has_player_flag(flags, PLAYER_FLAG_CANNOT_USE_COMBAT)
    }

    /// M1 — `CheckRight(target, INVULNERABLE)` equivalent — `crmain.cc:536-538`. Returns `true`
    /// when the target player has the `CannotBeAttacked` group flag (772 `INVULNERABLE` right),
    /// which zeroes incoming damage. Non-players are never invulnerable via this check
    /// (monsters use race-data immunities, handled separately in `combat_execute_with_stimulus`).
    pub(crate) fn player_is_invulnerable(&self, target: CreatureId) -> bool {
        let flags = self.player_group_flags(target);
        has_player_flag(flags, PLAYER_FLAG_CANNOT_BE_ATTACKED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PvpConfig;
    use crate::creature::CreatureKind;
    use crate::ids::CreatureId;
    use tfs_rust_common::WorldType;

    /// Helper: build a minimal `GameWorld` with the given `WorldType` for PVP-gate tests.
    fn make_pvp_world(world_type: WorldType) -> GameWorld {
        let mut world = crate::sim_harness::minimal_world();
        world.pvp_config = PvpConfig {
            world_type,
            protection_level: 1,
        };
        world
    }

    fn insert_player(world: &mut GameWorld, name: &str) -> CreatureId {
        let pos = tfs_rust_common::Position::new(0, 0, 7);
        let mut player = crate::sim_harness::test_player(name, pos);
        player.guid = name.len() as u32; // unique guid per player
        world.creatures.insert(CreatureKind::Player(player))
    }

    #[test]
    fn secure_mode_blocks_in_pvp_world() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let t = insert_player(&mut world, "bob");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(world.player_secure_mode_blocks_attack(a, t));
    }

    #[test]
    fn secure_mode_does_not_block_in_no_pvp_world() {
        let mut world = make_pvp_world(WorldType::NoPvp);
        let a = insert_player(&mut world, "alice");
        let t = insert_player(&mut world, "bob");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        // WorldType != Pvp → secure mode gate does not fire.
        assert!(!world.player_secure_mode_blocks_attack(a, t));
    }

    #[test]
    fn secure_mode_off_does_not_block() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let t = insert_player(&mut world, "bob");
        // secure_mode defaults to false.
        assert!(!world.player_secure_mode_blocks_attack(a, t));
    }

    #[test]
    fn secure_mode_does_not_block_monster_target() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let m = world.creatures.insert(CreatureKind::Monster(
            crate::creature::Monster::new(
                crate::sim_harness::minimal_creature_base(),
                tfs_rust_common::Position::new(1, 1, 7),
            ),
        ));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(!world.player_secure_mode_blocks_attack(a, m));
    }

    #[test]
    fn block_logout_extends_earliest_logout_round() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        world.player_block_logout(a, 60, false);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.earliest_logout_round, 160);
            assert_eq!(p.earliest_protection_zone_round, 0);
        }
    }

    #[test]
    fn block_logout_with_pz_block_extends_protection_zone_round() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        world.player_block_logout(a, 60, true);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.earliest_logout_round, 160);
            assert_eq!(p.earliest_protection_zone_round, 160);
        }
    }

    #[test]
    fn block_logout_pz_block_cleared_in_no_pvp_world() {
        let mut world = make_pvp_world(WorldType::NoPvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        // block_pz=true but WorldType == NoPvp → PZ block cleared.
        world.player_block_logout(a, 60, true);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.earliest_logout_round, 160);
            assert_eq!(p.earliest_protection_zone_round, 0);
        }
    }

    #[test]
    fn block_logout_takes_max_of_existing_and_new() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.earliest_logout_round = 200;
        }
        world.player_block_logout(a, 60, false);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            // max(200, 100+60=160) = 200.
            assert_eq!(p.earliest_logout_round, 200);
        }
    }

    #[test]
    fn set_fight_modes_writes_attack_mode_with_delay_on_change() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.server_ms = 5000;
        let a = insert_player(&mut world, "alice");
        // Default attack_mode is Balanced (wire byte 2). Change to Offensive (1).
        world.player_set_fight_modes(a, 1, 0, 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.attack_mode, FightMode::Offensive);
            // DelayAttack(2000) on change — `crcombat.cc:334`.
            assert_eq!(p.base.earliest_attack_ms, 7000);
        }
    }

    #[test]
    fn set_fight_modes_no_delay_when_attack_mode_unchanged() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.server_ms = 5000;
        let a = insert_player(&mut world, "alice");
        // Default is Balanced (2). Setting Balanced again → no DelayAttack.
        world.player_set_fight_modes(a, 2, 0, 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.attack_mode, FightMode::Balanced);
            assert_eq!(p.base.earliest_attack_ms, 0);
        }
    }

    #[test]
    fn set_fight_modes_writes_secure_mode() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        world.player_set_fight_modes(a, 2, 0, 1);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert!(p.secure_mode);
        }
        // Toggle back off.
        world.player_set_fight_modes(a, 2, 0, 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert!(!p.secure_mode);
        }
    }

    #[test]
    fn set_fight_modes_does_not_override_chase_when_following() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        // Simulate active follow → chase_mode forced to Close.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.base.follow_target = Some(a); // dummy target
            p.base.chase_mode = ChaseMode::Close;
        }
        // Player sends chase_mode = NONE (0); should not override Close while following.
        world.player_set_fight_modes(a, 2, 0, 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.base.chase_mode, ChaseMode::Close);
        }
    }

    #[test]
    fn invulnerable_check_false_for_empty_groups() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        // Empty GroupDatabase → no flags → not invulnerable.
        assert!(!world.player_is_invulnerable(a));
    }

    #[test]
    fn attack_blocked_by_right_false_for_empty_groups() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        assert!(!world.player_attack_blocked_by_right(a));
    }
}
