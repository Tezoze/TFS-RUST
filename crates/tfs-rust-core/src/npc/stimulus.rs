//! Same-floor NPC speech-stimulus candidate collection.

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use tfs_rust_common::Position;

/// Collect NPCs that should receive a normal-say talk stimulus from `speaker`.
///
/// Order follows [`crate::map::grid::CreatureGrid::collect_spectators_sector_order`]
/// (772 `TFindCreatures` block scan stand-in). Same-floor only; speaker skipped.
///
/// C++ `Talk` NPC fan-out — `operate.cc:2451-2468`.
pub fn collect_npc_speech_candidates(
    world: &GameWorld,
    speaker: CreatureId,
    speaker_pos: Position,
) -> Vec<CreatureId> {
    let range_x = world.mechanics.profile.npc.speech_range_x;
    let range_y = world.mechanics.profile.npc.speech_range_y;
    let mut raw = Vec::new();
    world.map.grid.collect_spectators_sector_order(
        speaker_pos.x,
        speaker_pos.y,
        speaker_pos.z,
        range_x,
        range_y,
        &mut raw,
    );

    let mut out = Vec::new();
    for cid in raw {
        if cid == speaker {
            continue;
        }
        let Some(kind) = world.creatures.get(cid) else {
            continue;
        };
        let CreatureKind::Npc(npc) = kind else {
            continue;
        };
        if npc.base.position.z != speaker_pos.z {
            continue;
        }
        // Exact XY box filter (sector order can include out-of-range tiles at sector edges).
        let dx = (npc.base.position.x as i32 - speaker_pos.x as i32).unsigned_abs();
        let dy = (npc.base.position.y as i32 - speaker_pos.y as i32).unsigned_abs();
        if dx > u32::from(range_x) || dy > u32::from(range_y) {
            continue;
        }
        out.push(cid);
    }
    out
}
