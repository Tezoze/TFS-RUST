//! First game-protocol burst after `Player` is placed (`ProtocolGame::sendAddCreature` self branch + map).
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::sendAddCreature` (player), `sendMapDescription`, `sendWorldLight`, `sendFightModes`.

use tfs_rust_common::ConnId;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Enqueue initial packets for a freshly loaded player (same order as C++: 0x17 → 0x0A → 0x0F → map → light → fight modes).
pub fn enqueue_initial_login_packets(
    world: &mut GameWorld,
    conn_id: ConnId,
    creature_id: CreatureId,
) {
    let Some((player_id, pos)) = world.creatures.get(creature_id).and_then(|k| match k {
        CreatureKind::Player(p) => Some((p.guid, p.base.position)),
        _ => None,
    }) else {
        return;
    };
    let pid = player_id;
    world.enqueue_outgoing(
        conn_id,
        tfs_rust_net::outgoing_extra::send_self_appear_login(pid).into_bytes(),
    );
    world.enqueue_outgoing(
        conn_id,
        tfs_rust_net::outgoing_extra::send_pending_state_entered().into_bytes(),
    );
    world.enqueue_outgoing(
        conn_id,
        tfs_rust_net::outgoing_extra::send_enter_world().into_bytes(),
    );
    world.enqueue_outgoing(
        conn_id,
        tfs_rust_net::map_description::send_map_description_stub(pos, pos).into_bytes(),
    );
    world.enqueue_outgoing(
        conn_id,
        tfs_rust_net::outgoing_extra::send_world_light(250, 215, false).into_bytes(),
    );
    world.enqueue_outgoing(
        conn_id,
        tfs_rust_net::outgoing_extra::send_fight_modes(1, 0, 0, 0).into_bytes(),
    );
}
