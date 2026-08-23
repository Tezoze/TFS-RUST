//! Native spawn loot then Lua `onSpawn` mutate hook (data-pack Lua Phase 4).
//!
//! Ordering: allow-gate → place → roll loot (non-summons) → combat recompute →
//! `on_monster_spawned` → combat recompute again (equipment may have changed).
//!
//! Pack: `Monster:onSpawn` / `EventCallback(EVENT_CALLBACK_ONSPAWN)`.
//! Corpus: `TMonster::TMonster` spawn inventory (`crnonpl.cc:2050`).

use tfs_rust_content::monsters::MonsterType;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::lua_scope::fire_on_monster_spawned;

impl GameWorld {
    /// Finish a placed monster: roll spawn loot (unless summoned), then Lua mutate.
    pub(crate) fn finish_monster_spawn(
        &mut self,
        cid: CreatureId,
        mtype: &MonsterType,
        startup: bool,
        artificial: bool,
    ) {
        let is_summon = self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().master.is_some());
        if !is_summon {
            self.roll_monster_spawn_loot(cid, mtype);
            self.recompute_monster_combat_from_equipment(cid);
        }
        fire_on_monster_spawned(self, cid, startup, artificial);
        if matches!(self.creatures.get(cid), Some(CreatureKind::Monster(_))) {
            self.recompute_monster_combat_from_equipment(cid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_dispatcher::EventDispatcher;
    use crate::sim_harness::{insert_monster, minimal_world};
    use std::cell::Cell;
    use std::rc::Rc;
    use tfs_rust_common::Position;
    use tfs_rust_common::ScriptContext;
    use tfs_rust_content::monsters::{
        MonsterDefenses, MonsterOutfit, MonsterType, MonsterTypeFlags,
    };

    struct HookRc(Rc<Cell<u32>>);

    impl EventDispatcher for HookRc {
        fn on_monster_spawned(
            &self,
            _creature: CreatureId,
            _pos: Position,
            _startup: bool,
            _artificial: bool,
            _ctx: &dyn ScriptContext,
        ) {
            self.0.set(self.0.get() + 1);
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    fn empty_rat() -> MonsterType {
        MonsterType {
            name: "Rat".into(),
            filename: "rat.xml".into(),
            name_description: "a rat".into(),
            race: "blood".into(),
            experience: 5,
            speed: 27,
            health_now: 20,
            health_max: 20,
            outfit: MonsterOutfit {
                corpse_id: 2813,
                ..MonsterOutfit::default()
            },
            flags: MonsterTypeFlags::default(),
            mana_cost: 0,
            loot: Vec::new(),
            attack_spells: Vec::new(),
            defenses: MonsterDefenses {
                armor: Some(1),
                defense: Some(3),
                spells: Vec::new(),
                immunity_poison: false,
                immunity_fire: false,
                immunity_energy: false,
                immunity_life_drain: false,
                see_invisible: false,
                immunity_physical: false,
                immunity_paralyze: false,
                immunity_outfit: false,
            },
            max_summons: 0,
            summons: Vec::new(),
            talk_texts: Vec::new(),
        }
    }

    #[test]
    fn finish_monster_spawn_fires_spawned_hook() {
        let mut world = minimal_world();
        let cid = insert_monster(&mut world, "Rat", Position::new(50, 50, 7), 100);
        let shared = Rc::new(Cell::new(0u32));
        world.events = Box::new(HookRc(shared.clone()));
        world.finish_monster_spawn(cid, &empty_rat(), false, false);
        assert_eq!(shared.get(), 1);
    }
}
