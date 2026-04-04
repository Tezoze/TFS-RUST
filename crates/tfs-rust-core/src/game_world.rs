//! Central simulation state: entities, map, managers, DB handle.
// C++ reference: `Game` / `Map` ownership in `game.cpp`.

use std::sync::Arc;

use dashmap::DashMap;
use slotmap::SlotMap;

use tfs_rust_db::DbPool;

use crate::config::ConfigManager;
use crate::creature::CreatureKind;
use crate::decay::DecayManager;
use crate::event_dispatcher::EventDispatcher;
use crate::house::HouseManager;
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::map::Map;
use crate::spawn::SpawnManager;
use crate::stability::StabilityManager;
use crate::wildcard::WildcardTree;

pub struct GameWorld {
    pub creatures: SlotMap<CreatureId, CreatureKind>,
    pub items: SlotMap<ItemId, Item>,
    pub map: Map,
    pub events: Box<dyn EventDispatcher>,
    pub config: Arc<ConfigManager>,
    pub db: DbPool,
    /// GAME THREAD ONLY — insert/remove from IO threads must not be added without review.
    pub player_by_name: DashMap<String, CreatureId>,
    /// GAME THREAD ONLY — paired with `player_by_name`.
    pub player_by_guid: DashMap<u32, CreatureId>,
    pub decay: DecayManager,
    pub spawns: SpawnManager,
    pub houses: HouseManager,
    pub wildcards: WildcardTree,
    pub stability: StabilityManager,
    pub tick_counter: u64,
}

impl GameWorld {
    pub fn new(
        map: Map,
        events: Box<dyn EventDispatcher>,
        config: Arc<ConfigManager>,
        db: DbPool,
        spawns: SpawnManager,
    ) -> Self {
        Self {
            creatures: SlotMap::with_key(),
            items: SlotMap::with_key(),
            map,
            events,
            config,
            db,
            player_by_name: DashMap::new(),
            player_by_guid: DashMap::new(),
            decay: DecayManager::default(),
            spawns,
            houses: HouseManager::default(),
            wildcards: WildcardTree::default(),
            stability: StabilityManager::default(),
            tick_counter: 0,
        }
    }

    /// One simulation tick (~50 ms target).
    pub fn on_tick(&mut self) {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        let _ = self.decay.tick(self.tick_counter);
        self.spawns.tick(std::time::Instant::now());
        if self.tick_counter.is_multiple_of(5) {
            self.events.lua_gc_step();
        }
    }
}
