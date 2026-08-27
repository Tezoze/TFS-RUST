//! 772 `ProcessConnections` — idle kick and round tracking.
//!
//! C++ reference: `connections.cc:21` `TConnection::Process`, `connections.cc:53` `ResetTimer`.

use tfs_rust_common::ConnId;
use tfs_rust_common::GamePacket;
use tfs_rust_net::outgoing::{send_ping, send_text_message};

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// C++ `TALK_ADMIN_MESSAGE` (`enums.hh`).
const TALK_ADMIN_MESSAGE: u8 = 18;

/// Rounds without any command before proactive ping #1 — `connections.cc:24`.
const PING_ROUND_1: u32 = 30;
/// Rounds without any command before proactive ping #2 — `connections.cc:24`.
const PING_ROUND_2: u32 = 60;
/// Rounds without any command before dead-connection logout — `connections.cc:37`.
const COMMAND_TIMEOUT_ROUNDS: u32 = 90;

/// C++ `TConnection::Process` — `connections.cc:21-51`.
///
/// All timing is **round-counter based** (`RoundNr`), never wallclock:
/// - Ping at `LastCommand == 30 || == 60` (`connections.cc:24`).
/// - Idle warn at `LastAction == 900` (`connections.cc:29`).
/// - Idle kick at `LastAction >= 960` (`connections.cc:35`).
/// - Connection timeout logout at `LastCommand >= 90` (`connections.cc:37`).
impl GameWorld {
    /// C++ `TConnection::ResetTimer` — update command/action round stamps on incoming packets.
    pub(crate) fn player_reset_connection_rounds(
        &mut self,
        cid: CreatureId,
        counts_as_action: bool,
    ) {
        // Phase 4: 1098 defer deleted — both eras use 772 round tracking.
        let round = self.round_nr;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_command_round = round;
            if counts_as_action {
                p.last_action_round = round;
            }
        }
    }

    /// C++ `ProcessConnections` idle kick + round-based ping + connection timeout
    /// (`connections.cc:21-51`). Returns `(conn, stop_fight)` pairs to disconnect.
    ///
    /// Idle warn/kick rounds are config-driven via `kickIdlePlayerAfterMinutes`
    /// (`config.lua`): warning at `N*60` rounds, kick at `(N+1)*60` rounds. The proactive
    /// ping cadence (30/60) and the dead-connection timeout (round 90) are 772 engine
    /// constants. Idle kick → `Logout(..., StopFight=true)`; command timeout →
    /// `StopFight=false` (`connections.cc:35-38`).
    pub(crate) fn process_connections(&mut self) -> Vec<(ConnId, bool)> {
        // Phase 4: 1098 defer deleted — both eras use 772 ProcessConnections.
        let round = self.round_nr;
        let idle_warn_rounds = self.connection_config.idle_warn_rounds();
        let idle_kick_rounds = self.connection_config.idle_kick_rounds();
        let mut kick: Vec<(ConnId, bool)> = Vec::new();

        // CONNECTION_LOGIN — `connections.cc:42–44`.
        if self.game_state != crate::game_state::GameState::Normal {
            for conn_id in self.login_pending_conns.drain() {
                kick.push((conn_id, false));
            }
        }

        let online: Vec<(ConnId, CreatureId)> = self
            .conn_to_creature
            .iter()
            .map(|(&conn, &cid)| (conn, cid))
            .collect();

        for (conn_id, cid) in online {
            let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
                continue;
            };
            let last_command = round.saturating_sub(p.last_command_round);
            let last_action = round.saturating_sub(p.last_action_round);

            // C++ `LastCommand == 30 || LastCommand == 60` → `SendPing` (`connections.cc:24`).
            if last_command == PING_ROUND_1 || last_command == PING_ROUND_2 {
                self.enqueue_outgoing(conn_id, send_ping().into_bytes());
            }

            if last_action == idle_warn_rounds {
                let msg = format!(
                    "You have been idle for {} minutes. You will be disconnected in one minute if you are still idle then.",
                    self.connection_config.kick_idle_after_minutes
                );
                self.enqueue_outgoing(
                    conn_id,
                    send_text_message(TALK_ADMIN_MESSAGE, &msg).into_bytes(),
                );
            }
            if last_action >= idle_kick_rounds {
                // `Logout(0, true)` — StopFight (`connections.cc:35-36`).
                kick.push((conn_id, true));
            } else if last_command >= COMMAND_TIMEOUT_ROUNDS {
                // `Logout(0, false)` — keep fighting on map (`connections.cc:37-38`).
                kick.push((conn_id, false));
            }
        }

        kick
    }

    /// Current `(brightness, color)` for `0x82` — `GetAmbiente` plus `setWorldLight` override.
    pub(crate) fn current_world_light(&self) -> (u8, u8) {
        if let Some((level, color)) = self.world_light_override {
            return (level, color);
        }
        if !self.config.default_world_light().unwrap_or(true) {
            // TFS default day light when automatic world light is disabled.
            return (0xFA, 0xD7);
        }
        let wt = crate::world_light::world_time_from_local_clock();
        crate::world_light::ambient_from_world_time(wt)
    }

    /// C++ `SendAmbiente` on brightness change (`main.cc:361-372`).
    pub(crate) fn tick_ambient_light(&mut self) {
        let (brightness, color) = self.current_world_light();
        if brightness as i16 == self.last_ambiente_brightness {
            return;
        }
        self.last_ambiente_brightness = brightness as i16;
        let packet =
            tfs_rust_net::outgoing_extra::send_world_light(brightness, color, false).into_bytes();
        let conns: Vec<ConnId> = self.conn_to_creature.keys().copied().collect();
        for conn_id in conns {
            self.enqueue_outgoing(conn_id, packet.clone());
        }
    }

    /// TFS `setWorldLight(level, color)` — `gameserver/src/luascript.cpp:3132-3145`.
    pub(crate) fn set_world_light(&mut self, level: u8, color: u8) -> bool {
        if self.config.default_world_light().unwrap_or(true) {
            return false;
        }
        self.world_light_override = Some((level, color));
        self.last_ambiente_brightness = level as i16;
        let packet =
            tfs_rust_net::outgoing_extra::send_world_light(level, color, false).into_bytes();
        let conns: Vec<ConnId> = self.conn_to_creature.keys().copied().collect();
        for conn_id in conns {
            self.enqueue_outgoing(conn_id, packet.clone());
        }
        true
    }
}

/// Whether a client packet counts as player action for idle kick.
///
/// Mirrors `TConnection::ResetTimer` (`tibia-game-master/src/connections.cc:53-63`): the five
/// 772 opcodes that refresh `TimeStamp` but **not** `TimeStampAction` are
/// `CL_CMD_PING` (0x1E → `Ping`), `CL_CMD_GO_STOP` (0x69 → `StopAutoWalk`),
/// `CL_CMD_CANCEL` (0xBE → `CancelAttackAndFollow`), `CL_CMD_REFRESH_FIELD` (0xC9 → `UpdateTile`
/// in the shared decoder; the 772 "browse field" request) and `CL_CMD_REFRESH_CONTAINER`
/// (0xCA → `UpdateContainer`).
///
/// `Turn` (0x6F–0x72) and `PingBack` (0x1D, OTClient-only) are **not** in the C++ exemption list
/// and therefore do count as actions — turning to dodge the idle kick is not 772-faithful.
pub(crate) fn packet_counts_as_action(packet: &GamePacket) -> bool {
    !matches!(
        packet,
        GamePacket::Ping
            | GamePacket::StopAutoWalk
            | GamePacket::CancelAttackAndFollow
            | GamePacket::UpdateTile
            | GamePacket::UpdateContainer { .. }
    )
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::Position;

    use crate::creature::CreatureKind;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };

    #[test]
    fn idle_kick_at_960_rounds() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Idle", pos));
        world.register_conn_mapping(tfs_rust_common::ConnId(1), player);

        // `last_command_round` stays recent so only the idle-kick path is exercised.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_action_round = 0;
            p.last_command_round = 960;
        }
        world.round_nr = 960;
        let kick = world.process_connections();
        assert_eq!(kick.len(), 1);
        assert_eq!(kick[0].0, tfs_rust_common::ConnId(1));
        assert!(kick[0].1, "idle kick → StopFight=true");
    }

    /// Custom `kickIdlePlayerAfterMinutes = 10` → warn at 600, kick at 660.
    #[test]
    fn idle_kick_uses_config_minutes() {
        let mut world = beat_driven_test_world();
        world.connection_config.kick_idle_after_minutes = 10;
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Idle", pos));
        world.register_conn_mapping(tfs_rust_common::ConnId(1), player);

        // Keep `last_command_round` recent so the 90-round connection timeout
        // doesn't fire before the idle kick — we're testing idle timing only.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_action_round = 0;
            p.last_command_round = 0;
        }
        // 600 rounds = 10 min → warning, not kick yet (command still recent).
        world.round_nr = 600;
        // Refresh command round so connection-timeout (>=90) doesn't fire.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 600;
        }
        let kick = world.process_connections();
        assert!(kick.is_empty(), "warn at 600 must not kick");

        // 660 rounds = 11 min → kick (command still recent).
        world.round_nr = 660;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 660;
        }
        let kick = world.process_connections();
        assert_eq!(kick.len(), 1, "kick at 660 (10+1 min)");
    }

    /// C++ `LastCommand == 30` → `SendPing` (`connections.cc:24`).
    #[test]
    fn round_based_ping_at_30_rounds() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Pinged", pos));
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, player);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 0;
            p.last_action_round = 0;
        }
        world.round_nr = 30;
        let kick = world.process_connections();
        assert!(kick.is_empty(), "ping round must not kick");
        let outgoing = world.pending_outgoing.get(&conn);
        assert!(
            outgoing.is_some_and(|q| !q.is_empty()),
            "ping must be enqueued"
        );
    }

    /// C++ `LastCommand >= 90` → logout (`connections.cc:37`).
    #[test]
    fn connection_timeout_kick_at_90_rounds() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Timeout", pos));
        world.register_conn_mapping(tfs_rust_common::ConnId(1), player);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 0;
            p.last_action_round = 0;
        }
        world.round_nr = 90;
        let kick = world.process_connections();
        assert_eq!(
            kick.len(),
            1,
            "LastCommand >= 90 must trigger connection timeout"
        );
        assert_eq!(kick[0].1, false, "command timeout → StopFight=false");
    }

    /// `packet_counts_as_action` mirrors `TConnection::ResetTimer` (`connections.cc:53-63`):
    /// the five 772 exempt opcodes do **not** refresh `TimeStampAction`; everything else (including
    /// `Turn` and `PingBack`, which were previously mis-exempted) does.
    #[test]
    fn packet_counts_as_action_matches_772_reset_timer() {
        use tfs_rust_common::GamePacket;
        use tfs_rust_common::enums::Direction;
        use tfs_rust_common::game_packet::SayPayload;

        use super::packet_counts_as_action;

        // Exempt — must NOT count as action (matches C++ exemption list).
        let exempt = [
            GamePacket::Ping,
            GamePacket::StopAutoWalk,
            GamePacket::CancelAttackAndFollow,
            GamePacket::UpdateTile,
            GamePacket::UpdateContainer { cid: 0 },
        ];
        for p in exempt {
            assert!(
                !packet_counts_as_action(&p),
                "exempt packet {p:?} must not count as action"
            );
        }

        // Counts as action — previously mis-exempted or never exempt.
        let actionable = [
            GamePacket::PingBack,
            GamePacket::Turn(Direction::North),
            GamePacket::Turn(Direction::East),
            GamePacket::Move(Direction::South),
            GamePacket::Attack { creature_id: 1 },
            GamePacket::Say(SayPayload {
                speak_class: 0,
                channel_id: 0,
                receiver: String::new(),
                text: "hi".into(),
            }),
        ];
        for p in actionable {
            assert!(
                packet_counts_as_action(&p),
                "action packet {p:?} must count as action"
            );
        }
    }

    /// Dead-connection `StopFight=false` keeps the body on the map until combat lock ends.
    #[test]
    fn dead_connection_map_presence_until_logout_possible() {
        use crate::game_world_lifecycle::LogoutPossible;
        use crate::sim_harness::insert_monster;

        let mut world = beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        let mpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        let player = insert_player(&mut world, test_player("Ghost", ppos));
        let mon = insert_monster(&mut world, "Rat", mpos, 100);
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(mpos, mon);
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, player);

        world.round_nr = 100;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.attack_target = Some(mon);
            p.earliest_logout_round = 160; // combat lock
        }

        // Simulate Connection::Logout(0, false): clear conn then StartLogout(false, false).
        world.unregister_conn_mapping(conn);
        world.creature_begin_logout(player, false, false);

        let p = match world.creatures.get(player) {
            Some(CreatureKind::Player(p)) => p,
            _ => panic!("body must remain on map"),
        };
        assert!(p.logging_out);
        assert_eq!(
            p.base.attack_target,
            Some(mon),
            "StopFight=false keeps AttackDest"
        );
        assert_eq!(
            p.base.latest_attack_round, 160,
            "StopAttack(60) from RoundNr 100"
        );
        assert_eq!(world.player_logout_possible(player), LogoutPossible::Combat);

        world.process_creatures();
        assert!(
            world.creatures.get(player).is_some(),
            "still combat-locked — ProcessCreatures must not remove"
        );

        world.round_nr = 160;
        // PK-mark clear + logout finalize on same ProcessCreatures pass.
        world.process_creatures();
        assert!(
            world.creatures.get(player).is_none(),
            "LogoutPossible after combat lock → remove body"
        );
    }

    /// Idle kick uses StopFight=true (`connections.cc:35-36`).
    #[test]
    fn idle_kick_uses_stop_fight_true() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Idle", pos));
        world.register_conn_mapping(tfs_rust_common::ConnId(1), player);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 0;
            p.last_action_round = 0;
        }
        // Default kickIdleAfterMinutes=15 → kick at 16*60=960.
        world.round_nr = 960;
        let kick = world.process_connections();
        assert_eq!(kick.len(), 1);
        assert!(kick[0].1, "idle kick → StopFight=true");
    }

    /// Relog TakeOver while deferred logout body is combat-locked (`connections.cc:231-252`).
    #[test]
    fn relog_takeover_while_combat_locked_logging_out() {
        use crate::sim_harness::insert_monster;

        let mut world = beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        let mpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        let mut pl = test_player("TakeOver", ppos);
        pl.guid = 42;
        let player = insert_player(&mut world, pl);
        let mon = insert_monster(&mut world, "Rat", mpos, 100);
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(mpos, mon);
        world.player_by_guid.insert(42, player);
        world.player_by_name.insert("TakeOver".into(), player);

        world.round_nr = 100;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.attack_target = Some(mon);
            p.earliest_logout_round = 160;
        }
        world.creature_begin_logout(player, false, false);
        assert!(matches!(
            world.creatures.get(player),
            Some(CreatureKind::Player(p)) if p.logging_out
        ));

        let (cid, old_conn) = world
            .player_try_takeover_for_login(42, "TakeOver", 0, 0)
            .expect("takeover must succeed while combat-locked")
            .expect("existing body");
        assert_eq!(cid, player);
        assert!(old_conn.is_none(), "no prior TCP on deferred body");

        let p = match world.creatures.get(player) {
            Some(CreatureKind::Player(p)) => p,
            _ => panic!("same body must remain"),
        };
        assert!(!p.logging_out);
        assert!(!p.logout_allowed);
        assert!(p.base.attack_target.is_none(), "TakeOver StopAttack(0)");
        assert_eq!(world.player_by_guid.get(&42), Some(&player));
        assert_eq!(world.creatures.len(), 2, "no duplicate player spawn");
    }

    /// TakeOver rejects when LoggingOut and LogoutPossible (`connections.cc:238-241`).
    #[test]
    fn relog_rejected_when_logout_finalize_ready() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let mut pl = test_player("Finishing", pos);
        pl.guid = 7;
        let player = insert_player(&mut world, pl);
        world.map.register_creature_at(pos, player);
        world.player_by_guid.insert(7, player);
        world.player_by_name.insert("Finishing".into(), player);

        world.round_nr = 200;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.earliest_logout_round = 0;
            p.logging_out = true;
        }
        let err = world
            .player_try_takeover_for_login(7, "Finishing", 0, 0)
            .expect_err("must reject finalize-ready logout");
        assert!(err.to_string().contains("logging out"), "err={err}");
        assert!(world.creatures.get(player).is_some());
    }

    /// TakeOver while still connected clears old mapping (`connections.cc:244-252`).
    #[test]
    fn takeover_detaches_old_connection() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let mut pl = test_player("KickOld", pos);
        pl.guid = 9;
        let player = insert_player(&mut world, pl);
        world.map.register_creature_at(pos, player);
        world.player_by_guid.insert(9, player);
        let old = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(old, player);

        let (cid, old_conn) = world
            .player_try_takeover_for_login(9, "KickOld", 1, 2)
            .unwrap()
            .unwrap();
        assert_eq!(cid, player);
        assert_eq!(old_conn, Some(old));
        assert!(world.conn_to_creature.get(&old).is_none());
        assert!(world.creature_to_conn.get(&player).is_none());
        if let Some(CreatureKind::Player(p)) = world.creatures.get(player) {
            assert_eq!(p.operating_system, 1);
            assert_eq!(p.otclient_v8, 2);
            assert!(!p.logging_out);
        }
    }

    #[test]
    fn login_connection_disconnects_when_not_ok() {
        let mut world = beat_driven_test_world();
        let conn = tfs_rust_common::ConnId(9);
        world.login_pending_conns.insert(conn);
        world.game_state = crate::game_state::GameState::Closed;
        let kick = world.process_connections();
        assert_eq!(kick.len(), 1);
        assert_eq!(kick[0].0, conn);
        assert!(!kick[0].1, "login disconnect is not StopFight idle kick");
        assert!(world.login_pending_conns.is_empty());
    }
}
