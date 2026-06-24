//! Minimal `GameWorld` builder for unit tests (never touches the database).
#[cfg(test)]
pub mod support {
    pub use crate::sim_harness::*;

    use std::collections::HashMap;

    use crate::event_dispatcher::EventDispatcher;
    use crate::game_world::GameWorld;
    use crate::ids::CreatureId;

    /// Test helper — counts `EventDispatcher::on_think` calls per creature.
    #[derive(Debug, Default)]
    pub struct CountingEventDispatcher {
        think_calls: std::sync::Mutex<HashMap<CreatureId, u32>>,
        intervals: std::sync::Mutex<Vec<u32>>,
    }

    impl CountingEventDispatcher {
        pub fn total_think_calls(&self) -> u32 {
            self.think_calls.lock().expect("lock").values().sum()
        }

        pub fn intervals(&self) -> Vec<u32> {
            self.intervals.lock().expect("lock").clone()
        }
    }

    impl EventDispatcher for CountingEventDispatcher {
        fn on_think(&self, creature: CreatureId, interval_ms: u32) {
            *self
                .think_calls
                .lock()
                .expect("lock")
                .entry(creature)
                .or_insert(0) += 1;
            self.intervals.lock().expect("lock").push(interval_ms);
        }
    }
}
