//! Runtime creature discriminant.
// C++ reference: `dynamic_cast` between `Player` / `Monster` / `Npc`.

use crate::creature::monster::Monster;
use crate::creature::npc::Npc;
use crate::creature::player::Player;
use crate::ids::CreatureId;
use tfs_rust_common::Position;

#[derive(Debug)]
pub enum CreatureKind {
    Player(Player),
    Monster(Monster),
    Npc(Npc),
}

impl CreatureKind {
    pub fn position(&self) -> Position {
        match self {
            CreatureKind::Player(p) => p.base.position,
            CreatureKind::Monster(m) => m.base.position,
            CreatureKind::Npc(n) => n.base.position,
        }
    }

    pub fn set_position(&mut self, pos: Position) {
        match self {
            CreatureKind::Player(p) => p.base.position = pos,
            CreatureKind::Monster(m) => m.base.position = pos,
            CreatureKind::Npc(n) => n.base.position = pos,
        }
    }

    pub fn base_mut(&mut self) -> &mut crate::creature::base::CreatureBase {
        match self {
            CreatureKind::Player(p) => &mut p.base,
            CreatureKind::Monster(m) => &mut m.base,
            CreatureKind::Npc(n) => &mut n.base,
        }
    }

    pub fn is_summon(&self) -> bool {
        match self {
            CreatureKind::Player(p) => p.base.is_summon(),
            CreatureKind::Monster(m) => m.base.is_summon(),
            CreatureKind::Npc(n) => n.base.is_summon(),
        }
    }
}

pub(crate) fn creature_id_eq_slice(ids: &[CreatureId], needle: CreatureId) -> Option<usize> {
    ids.iter().position(|&id| id == needle)
}
