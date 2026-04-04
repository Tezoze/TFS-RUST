//! Central simulation state: entities, map, managers, DB handle.
// C++ reference: `Game` / `Map` ownership in `game.cpp`.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use slotmap::SlotMap;

use tfs_rust_db::DbPool;

use crate::config::ConfigManager;
use crate::creature::CreatureKind;
use crate::decay::DecayManager;
use crate::event_dispatcher::EventDispatcher;
use crate::guild::GuildRegistry;
use crate::house::HouseManager;
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::map::Map;
use crate::party::{Party, PartyInviteState};
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
    pub guilds: GuildRegistry,
    pub parties: HashMap<u32, Party>,
    pub party_invites: PartyInviteState,
    pub next_party_id: u32,
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
            guilds: GuildRegistry::default(),
            parties: HashMap::new(),
            party_invites: PartyInviteState::default(),
            next_party_id: 1,
            decay: DecayManager::default(),
            spawns,
            houses: HouseManager::default(),
            wildcards: WildcardTree::default(),
            stability: StabilityManager::default(),
            tick_counter: 0,
        }
    }

    /// Remove creature from map index, player lookups, guild online; remove summons if master dies.
    // C++ reference: `Game::removeCreature` — summon chain.
    pub fn remove_creature(&mut self, id: CreatureId) {
        let mut summons: Vec<CreatureId> = Vec::new();
        for (cid, k) in self.creatures.iter() {
            let m = match k {
                CreatureKind::Player(p) => p.base.master,
                CreatureKind::Monster(mo) => mo.base.master,
                CreatureKind::Npc(n) => n.base.master,
            };
            if m == Some(id) {
                summons.push(cid);
            }
        }
        for s in summons {
            self.remove_creature(s);
        }

        let pos = self.creatures.get(id).map(|k| k.position());
        let player_cleanup = self.creatures.get(id).and_then(|k| {
            if let CreatureKind::Player(pl) = k {
                Some((pl.base.name.clone(), pl.guid, pl.social.guild_id.is_some()))
            } else {
                None
            }
        });

        if let Some(p) = pos {
            self.map.unregister_creature_index(p, id);
        }

        if let Some((name, guid, in_guild)) = player_cleanup {
            self.player_by_name.remove(&name);
            self.player_by_guid.remove(&guid);
            if in_guild {
                self.guilds.unregister_online(id);
            }
        }

        self.creatures.remove(id);
    }

    /// Run death XP / events / corpse scheduling, then remove the creature (and summons).
    pub fn apply_creature_death(&mut self, victim: CreatureId) {
        crate::death::handle_creature_death(
            &mut self.creatures,
            &mut self.items,
            &mut self.decay,
            self.events.as_ref(),
            victim,
            self.tick_counter,
            None,
        );
        self.remove_creature(victim);
    }

    /// One simulation tick (~50 ms target).
    pub fn on_tick(&mut self) {
        self.tick_counter = self.tick_counter.wrapping_add(1);

        let tick = self.tick_counter;
        for (cid, k) in self.creatures.iter_mut() {
            if let CreatureKind::Monster(m) = k {
                m.think_tick(tick, cid);
            }
        }

        let _ = self.decay.tick(self.tick_counter);
        self.spawns.tick(std::time::Instant::now());
        if self.tick_counter.is_multiple_of(5) {
            self.events.lua_gc_step();
        }
    }
}
