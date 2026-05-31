//! Runtime creature discriminant.
// C++ reference: `dynamic_cast` between `Player` / `Monster` / `Npc`.

use crate::creature::monster::Monster;
use crate::creature::npc::Npc;
use crate::creature::player::Player;
use crate::ids::CreatureId;
use tfs_rust_common::Position;

#[derive(Debug)]
// `Player` is the largest variant, but boxing it would add indirection to the hottest
// creature-access path. Entity storage is intentionally contiguous / no-indirection
// (see steering: tfs-entity-storage). Keep inline.
#[allow(clippy::large_enum_variant)]
pub enum CreatureKind {
    Player(Player),
    Monster(Monster),
    Npc(Npc),
}

impl CreatureKind {
    pub fn position(&self) -> Position {
        self.base().position
    }

    pub fn set_position(&mut self, pos: Position) {
        self.base_mut().position = pos;
    }

    pub fn base(&self) -> &crate::creature::base::CreatureBase {
        match self {
            CreatureKind::Player(p) => &p.base,
            CreatureKind::Monster(m) => &m.base,
            CreatureKind::Npc(n) => &n.base,
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
        self.base().is_summon()
    }
}

pub(crate) fn creature_id_eq_slice(ids: &[CreatureId], needle: CreatureId) -> Option<usize> {
    ids.iter().position(|&id| id == needle)
}
