//! 16×16 XY sector creature lists spanning all floors.
//!
//! Pack / corpus: `TFindCreatures` `blockx`/`blocky` 16 — `crmain.cc:101–144`.
//! Lists are `Vec<CreatureId>` (not `NextChainCreature` linked lists). 64×64 chunks
//! stay tile storage only.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::CreatureId;

use super::grid::SECTOR_SIZE;

/// Packed `(blockx, blocky)` — no floor; chain membership spans Z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SectorKey(u32);

impl SectorKey {
    #[inline]
    pub fn from_xy(x: u16, y: u16) -> Self {
        let bx = u32::from(x / SECTOR_SIZE);
        let by = u32::from(y / SECTOR_SIZE);
        SectorKey(by << 16 | bx)
    }

    #[inline]
    fn from_block(bx: u16, by: u16) -> Self {
        SectorKey(u32::from(by) << 16 | u32::from(bx))
    }
}

#[derive(Debug, Default)]
struct SectorLists {
    creatures: Vec<CreatureId>,
    players: Vec<CreatureId>,
}

/// XY sector index: creatures + optional player split.
#[derive(Debug, Default)]
pub(crate) struct SectorIndex {
    sectors: FxHashMap<SectorKey, SectorLists>,
    /// Survives unregister so a walk (`unregister` + `register`) keeps player membership.
    player_sector: FxHashMap<CreatureId, SectorKey>,
    player_ids: FxHashSet<CreatureId>,
}

impl SectorIndex {
    #[inline]
    fn lists_mut(&mut self, key: SectorKey) -> &mut SectorLists {
        self.sectors.entry(key).or_default()
    }

    fn push_unique(list: &mut Vec<CreatureId>, id: CreatureId) {
        if !list.contains(&id) {
            list.push(id);
        }
    }

    fn drop_empty(&mut self, key: SectorKey) {
        let empty = self
            .sectors
            .get(&key)
            .is_some_and(|s| s.creatures.is_empty() && s.players.is_empty());
        if empty {
            self.sectors.remove(&key);
        }
    }

    /// Insert into the XY sector covering `(x, y)`. `is_player` or prior
    /// [`Self::note_player`] membership also updates the player list.
    pub fn insert(&mut self, x: u16, y: u16, id: CreatureId, is_player: bool) {
        let key = SectorKey::from_xy(x, y);
        if is_player {
            self.player_ids.insert(id);
        }
        let as_player = is_player || self.player_ids.contains(&id);
        if as_player {
            if let Some(old) = self.player_sector.insert(id, key)
                && old != key
            {
                if let Some(prev) = self.sectors.get_mut(&old) {
                    prev.players.retain(|c| *c != id);
                }
                self.drop_empty(old);
            }
        }

        let lists = self.lists_mut(key);
        Self::push_unique(&mut lists.creatures, id);
        if as_player {
            Self::push_unique(&mut lists.players, id);
        }
    }

    /// Remove from this XY sector's lists. Player membership is kept until
    /// [`Self::forget_player`] so a subsequent `insert` at a new XY re-joins.
    pub fn remove(&mut self, x: u16, y: u16, id: CreatureId) {
        let key = SectorKey::from_xy(x, y);
        let Some(lists) = self.sectors.get_mut(&key) else {
            return;
        };
        lists.creatures.retain(|c| *c != id);
        lists.players.retain(|c| *c != id);
        self.drop_empty(key);
    }

    /// Mark `id` as a player at `(x, y)` (conn mapping; may precede tile register).
    pub fn note_player(&mut self, x: u16, y: u16, id: CreatureId) {
        self.player_ids.insert(id);
        let key = SectorKey::from_xy(x, y);
        if let Some(old) = self.player_sector.insert(id, key)
            && old != key
        {
            if let Some(prev) = self.sectors.get_mut(&old) {
                prev.players.retain(|c| *c != id);
            }
            self.drop_empty(old);
        }
        let lists = self.lists_mut(key);
        Self::push_unique(&mut lists.players, id);
    }

    /// Drop player-list membership (disconnect). Creature list is unchanged.
    pub fn forget_player(&mut self, id: CreatureId) {
        self.player_ids.remove(&id);
        if let Some(key) = self.player_sector.remove(&id)
            && let Some(lists) = self.sectors.get_mut(&key)
        {
            lists.players.retain(|c| *c != id);
            self.drop_empty(key);
        }
    }

    /// Append sector lists overlapping the XY box, `blocky` outer / `blockx` inner
    /// (`crmain.cc:101–144`). All floors. Callers filter Z / `canSee`.
    pub fn collect(
        &self,
        center_x: u16,
        center_y: u16,
        range_x: u16,
        range_y: u16,
        players_only: bool,
        out: &mut Vec<CreatureId>,
    ) {
        let x0 = center_x.saturating_sub(range_x);
        let y0 = center_y.saturating_sub(range_y);
        let x1 = center_x.saturating_add(range_x);
        let y1 = center_y.saturating_add(range_y);

        let bx0 = x0 / SECTOR_SIZE;
        let by0 = y0 / SECTOR_SIZE;
        let bx1 = x1 / SECTOR_SIZE;
        let by1 = y1 / SECTOR_SIZE;

        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                let key = SectorKey::from_block(bx, by);
                let Some(lists) = self.sectors.get(&key) else {
                    continue;
                };
                let src = if players_only {
                    lists.players.as_slice()
                } else {
                    lists.creatures.as_slice()
                };
                if !src.is_empty() {
                    out.extend_from_slice(src);
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn contains_creature(&self, x: u16, y: u16, id: CreatureId) -> bool {
        self.sectors
            .get(&SectorKey::from_xy(x, y))
            .is_some_and(|s| s.creatures.contains(&id))
    }

    #[allow(dead_code)]
    pub fn for_each_creature(&self, mut f: impl FnMut(SectorKey, CreatureId)) {
        for (&key, lists) in &self.sectors {
            for &id in &lists.creatures {
                f(key, id);
            }
        }
    }
}
