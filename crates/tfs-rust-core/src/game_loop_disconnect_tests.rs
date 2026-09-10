//! `PlayerDisconnect.stop_fight` — socket drop vs `CL_CMD_LOGOUT`.
//! C++ reference: `connections.cc:37` `Logout(0, false)`; `crmain.cc:414` `StartLogout`.

use std::collections::{HashMap, HashSet, VecDeque};

use tfs_rust_common::game_packet::GamePacket;
use tfs_rust_common::{ConnId, GameCommand, Position};

use crate::creature::CreatureKind;
use crate::sim_harness::{
    TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, insert_monster, insert_player, test_player,
};
use crate::test_world::support::ensure_walkable_tile;

use super::handle_player_disconnect;

fn combat_locked_attacker() -> (
    crate::game_world::GameWorld,
    ConnId,
    crate::ids::CreatureId,
    crate::ids::CreatureId,
) {
    let mut world = beat_driven_test_world();
    let ppos = Position::new(100, 100, 7);
    let mpos = Position::new(101, 100, 7);
    ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
    ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
    let pid = insert_player(&mut world, test_player("Hero", ppos));
    let mon = insert_monster(&mut world, "Rat", mpos, 100);
    world.map.register_creature_at(ppos, pid);
    world.map.register_creature_at(mpos, mon);
    let conn = ConnId(1);
    world.register_conn_mapping(conn, pid);
    world.round_nr = 100;
    if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(pid) {
        p.base.attack_target = Some(mon);
        p.earliest_logout_round = 160;
    }
    (world, conn, pid, mon)
}

#[tokio::test(flavor = "current_thread")]
async fn socket_drop_keeps_attack_for_60_rounds() {
    let (mut world, conn, pid, mon) = combat_locked_attacker();
    let mut pending_login = HashSet::new();
    let mut sinks = HashMap::new();
    handle_player_disconnect(
        &mut world,
        &mut pending_login,
        conn,
        false,
        false,
        &mut sinks,
        &None,
    );
    let base = world
        .creatures
        .get(pid)
        .expect("combat-locked body stays")
        .base();
    assert_eq!(base.attack_target, Some(mon), "socket drop StopFight=false");
    assert_eq!(base.latest_attack_round, 160);
}

#[tokio::test(flavor = "current_thread")]
async fn logout_packet_clears_attack_now() {
    let (mut world, conn, pid, _mon) = combat_locked_attacker();
    let mut pending_login = HashSet::new();
    let mut sinks = HashMap::new();
    handle_player_disconnect(
        &mut world,
        &mut pending_login,
        conn,
        true,
        true,
        &mut sinks,
        &None,
    );
    let base = world
        .creatures
        .get(pid)
        .expect("combat-locked body stays")
        .base();
    assert!(
        base.attack_target.is_none(),
        "CL_CMD_LOGOUT StopFight=true clears dest now"
    );
}

#[test]
fn logout_packet_enqueues_stop_fight_true() {
    let mut world = beat_driven_test_world();
    let pos = Position::new(100, 100, 7);
    ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
    let pid = insert_player(&mut world, test_player("Quit", pos));
    world.map.register_creature_at(pos, pid);
    let conn = ConnId(1);
    world.register_conn_mapping(conn, pid);

    let (_tx, mut game_rx, _ctrl_rx) = tfs_rust_net::open_game_command_channels();
    let mut pending = VecDeque::new();
    super::handle_game_packet(
        &mut world,
        conn,
        GamePacket::Logout,
        &mut game_rx,
        &mut pending,
    );
    match pending.pop_front() {
        Some((_, GameCommand::PlayerDisconnect { stop_fight, .. })) => {
            assert!(stop_fight, "CL_CMD_LOGOUT → StopFight=true");
        }
        other => panic!("expected PlayerDisconnect, got {other:?}"),
    }
}
