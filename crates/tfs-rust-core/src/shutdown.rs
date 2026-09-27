//! `LogoutAllPlayers` + `RefreshMap` on reboot fire and on SIGINT.
//! C++ reference: `crplayer.cc:1874` `LogoutAllPlayers`; `main.cc:423-429`.
//! SIGINT in the decompile does not refresh (`main.cc:89-96`); this server does.

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::game_world_lifecycle::LogoutPossible;

/// 772 `LogoutAllPlayers` — delete every player now (`crplayer.cc:1874-1892`).
/// Not deferred to `ProcessCreatures`.
pub fn logout_all_players(world: &mut GameWorld) {
    let cids: Vec<_> = world
        .creatures
        .iter()
        .filter_map(|(id, k)| matches!(k, CreatureKind::Player(_)).then_some(id))
        .collect();
    for cid in cids {
        world.broadcast_player_logout_poff(cid);
        world.creature_begin_logout(cid, true, true);
        if let Some(conn) = world.creature_to_conn.get(&cid).copied() {
            world.unregister_conn_mapping(conn);
            world.forget_known_creatures_for_conn(conn);
        }
        if world.player_logout_possible(cid) == LogoutPossible::Ok {
            world.remove_creature(cid);
        }
    }
}

/// Logout everyone, then `RefreshMap` when this fire is a reboot (`main.cc:427-429`).
pub fn run(world: &mut GameWorld, reboot: bool) {
    logout_all_players(world);
    if reboot {
        let _ = world.refresh_map();
    }
}

/// Ctrl+C: logout, then restore every snapshotted refresh tile before the live-map write.
///
/// Decompile `DefaultHandler` ends the process without `RefreshMap` (`main.cc:89-96`).
pub fn run_interrupt(world: &mut GameWorld) {
    logout_all_players(world);
    let _ = world.refresh_map();
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::Position;

    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };

    #[test]
    fn shutdown_removes_players_online_rows() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let cid = insert_player(&mut world, test_player("Saver", pos));
        world.register_conn_mapping(tfs_rust_common::ConnId(1), cid);
        world.player_by_guid.insert(1, cid);
        assert_eq!(world.player_by_guid.len(), 1);
        super::logout_all_players(&mut world);
        assert!(
            world.player_by_guid.is_empty(),
            "LogoutAllPlayers must drop in-memory players (players_online delete is spawned)"
        );
        assert!(world.creatures.get(cid).is_none());
    }

    #[test]
    fn interrupt_restores_refresh_tile() {
        use crate::cylinder::CylinderFlags;
        use crate::item::Item;
        use crate::sector_refresh::TileRefreshSnap;
        use crate::tile::flags as tile_flags;

        let mut world = beat_driven_test_world();
        let pos = Position::new(80, 80, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        if let Some(t) = world.map.get_tile_mut(pos) {
            t.body_mut().flags |= tile_flags::REFRESH;
        }
        world.map.refresh_snapshots.insert(
            pos,
            TileRefreshSnap::from_tile(world.map.get_tile(pos).unwrap().body(), &world.items),
        );
        let junk = world.items.insert(Item::new_single(3031));
        world
            .internal_add_item_to_tile(pos, junk, CylinderFlags::NO_LIMIT)
            .expect("drop");
        super::run_interrupt(&mut world);
        let body = world.map.get_tile(pos).unwrap().body();
        assert!(!body.down_items().contains(&junk) && !body.top_items().contains(&junk));
        assert!(world.items.get(junk).is_none());
    }
}
