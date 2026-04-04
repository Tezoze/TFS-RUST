//! Map tile stacks (ground, items, creatures) and flags.
// C++ reference: `Tile` (`tile.h`), `Tile::queryAdd`, `queryRemove`, `addThing`, `removeThing`.

use crate::ids::CreatureId;
use tfs_rust_common::{enums::ZoneType, Position};

/// OTBM / TFS tile bitfield (subset; extend as engine grows).
pub mod flags {
    pub const NONE: u32 = 0;
    /// Blocks line of sight / projectiles when combined with item blocking (simplified).
    pub const BLOCK_SOLID: u32 = 1 << 0;
    pub const BLOCK_PROJECTILE: u32 = 1 << 1;
}

#[derive(Debug, Clone)]
pub struct TileBody {
    pub position: Position,
    pub ground: Option<u16>,
    pub items: Vec<u16>,
    pub creatures: Vec<CreatureId>,
    pub flags: u32,
    pub zone: ZoneType,
}

#[derive(Debug, Clone)]
pub struct HouseTile {
    pub inner: TileBody,
    /// House identifier from OTBM / `houses` table.
    pub house_id: u32,
}

#[derive(Debug, Clone)]
pub enum Tile {
    Normal(TileBody),
    House(HouseTile),
}

impl Tile {
    pub fn position(&self) -> Position {
        match self {
            Tile::Normal(t) => t.position,
            Tile::House(h) => h.inner.position,
        }
    }

    pub fn body_mut(&mut self) -> &mut TileBody {
        match self {
            Tile::Normal(t) => t,
            Tile::House(h) => &mut h.inner,
        }
    }

    pub fn body(&self) -> &TileBody {
        match self {
            Tile::Normal(t) => t,
            Tile::House(h) => &h.inner,
        }
    }

    /// TFS `Tile::queryAdd` — minimal placeholder until item database + walkability rules land.
    pub fn query_add(&self, _thing_size: u8) -> bool {
        // C++ reference: `tile.cpp` Tile::queryAdd — checks blocking, height, etc.
        true
    }

    pub fn query_remove(&self, _thing_size: u8) -> bool {
        true
    }

    pub fn add_creature(&mut self, id: CreatureId) {
        self.body_mut().creatures.push(id);
    }

    pub fn remove_creature(&mut self, id: CreatureId) -> bool {
        let body = self.body_mut();
        if let Some(i) = body.creatures.iter().position(|&c| c == id) {
            body.creatures.swap_remove(i);
            return true;
        }
        false
    }
}
