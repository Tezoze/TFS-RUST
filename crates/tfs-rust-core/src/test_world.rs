//! Minimal `GameWorld` builder for unit tests (never touches the database).
#[cfg(test)]
pub mod support {
    pub use crate::sim_harness::*;

    use std::collections::HashMap;

    use crate::creature::{MonsterAiConfig, MonsterSpell, SpellImpact, SpellShape};
    use crate::event_dispatcher::EventDispatcher;
    use crate::game_world::GameWorld;
    use crate::ids::CreatureId;
    use tfs_rust_common::enums::CombatType;

    /// Ranged spell fixture for 772 dist-idle tests (`ThrowPossible` / `DistanceFighting`).
    pub fn dist_idle_ranged_spell() -> MonsterSpell {
        MonsterSpell {
            delay: 2000,
            range: 7,
            radius: 0,
            min_cycle: 0,
            shape: SpellShape::Victim,
            impact: SpellImpact::Damage {
                element: CombatType::Physical,
                base: 10,
                variation: 10,
            },
            shoot_effect: None,
            area_effect: None,
        }
    }

    /// Hostile monster with in-band ranged spell — satisfies [`GameWorld::monster_throw_possible`].
    pub fn dist_idle_monster_config(target_distance: i32) -> MonsterAiConfig {
        let mut cfg = MonsterAiConfig {
            is_hostile: true,
            target_distance,
            ..MonsterAiConfig::default()
        };
        cfg.spells.push(dist_idle_ranged_spell());
        cfg
    }

    /// Test helper — counts `EventDispatcher::on_think` calls per creature.
    #[derive(Debug, Default)]
    pub struct CountingEventDispatcher {
        think_calls: std::sync::Mutex<HashMap<CreatureId, u32>>,
    }

    impl CountingEventDispatcher {
        pub fn total_think_calls(&self) -> u32 {
            self.think_calls.lock().expect("lock").values().sum()
        }
    }

    impl EventDispatcher for CountingEventDispatcher {
        fn on_think(&self, creature: CreatureId, _interval_ms: u32) {
            *self
                .think_calls
                .lock()
                .expect("lock")
                .entry(creature)
                .or_insert(0) += 1;
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
}
