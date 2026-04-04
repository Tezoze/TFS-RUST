//! Creature kinds stored in `GameWorld::creatures`.
// C++ reference: `Player` / `Monster` / `Npc` subclasses of `Creature`.

use crate::ids::CreatureId;
use tfs_rust_common::Position;

#[derive(Debug)]
pub struct PlayerStub {
    pub name: String,
    pub guid: u32,
    pub position: Position,
}

#[derive(Debug)]
pub struct MonsterStub {
    pub name: String,
    pub position: Position,
}

#[derive(Debug)]
pub struct NpcStub {
    pub name: String,
    pub position: Position,
}

#[derive(Debug)]
pub enum CreatureKind {
    Player(PlayerStub),
    Monster(MonsterStub),
    Npc(NpcStub),
}

impl CreatureKind {
    pub fn position(&self) -> Position {
        match self {
            CreatureKind::Player(p) => p.position,
            CreatureKind::Monster(m) => m.position,
            CreatureKind::Npc(n) => n.position,
        }
    }

    pub fn set_position(&mut self, pos: Position) {
        match self {
            CreatureKind::Player(p) => p.position = pos,
            CreatureKind::Monster(m) => m.position = pos,
            CreatureKind::Npc(n) => n.position = pos,
        }
    }
}

/// Used by spatial index when removing a creature from a leaf list.
pub(crate) fn creature_id_eq_slice(ids: &[CreatureId], needle: CreatureId) -> Option<usize> {
    ids.iter().position(|&id| id == needle)
}
