---
inclusion: auto
name: tfs-entity-storage
description: SlotMap-based entity storage patterns and ID type usage for safe, generational references.
---

# Entity Storage (SlotMap and ID Types)

All game entities (creatures, items) use `slotmap::SlotMap` for storage and typed keys for references.

## Core Architecture

```rust
use slotmap::{SlotMap, Key};
use crate::ids::{CreatureId, ItemId};

pub struct GameWorld {
    pub creatures: SlotMap<CreatureId, CreatureKind>,
    pub items: SlotMap<ItemId, Item>,
    // ...
}
```

## Never Store Direct References

**Bad:**
```rust
struct Player {
    target: &'a Creature,           // DON'T: lifetime hell
    target: Rc<RefCell<Creature>>,  // DON'T: runtime borrow checks
    target: Arc<Mutex<Creature>>,   // DON'T: unnecessary locking
}
```

**Good:**
```rust
struct Player {
    target: Option<CreatureId>,  // DO: typed SlotMap key
}
```

## Accessing Entities

Always handle the case where the entity no longer exists:

```rust
// Good: Safe access
if let Some(target) = world.creatures.get(player.target?) {
    // Use target
}

// Good: With context
let target = world.creatures.get(target_id)
    .ok_or_else(|| anyhow!("target creature not found"))?;

// BAD: Never use get_unchecked
let target = world.creatures.get_unchecked(target_id); // NEVER DO THIS
```

## Cross-Crate References

Pass IDs, not borrowed data:

```rust
// Good: Function takes ID
pub fn apply_damage(world: &mut GameWorld, target_id: CreatureId, damage: i32) {
    if let Some(target) = world.creatures.get_mut(target_id) {
        target.health -= damage;
    }
}

// Bad: Borrowing conflicts
pub fn apply_damage(target: &mut Creature, damage: i32) {
    // Can't access world while target is borrowed
}
```

## Iteration Patterns

```rust
// Good: Direct iteration when no mutation needed
for (id, creature) in world.creatures.iter() {
    println!("{:?}: {}", id, creature.name);
}

// Good: Collect IDs first if mutation during iteration
let dead_creatures: Vec<CreatureId> = world.creatures
    .iter()
    .filter(|(_, c)| c.health <= 0)
    .map(|(id, _)| id)
    .collect();

for id in dead_creatures {
    world.remove_creature(id);
}

// Bad: Mutation during iteration
for (id, creature) in world.creatures.iter_mut() {
    if creature.health <= 0 {
        world.remove_creature(id); // COMPILE ERROR: can't borrow world
    }
}
```

## ID Type Safety

Each entity type has its own key type:

```rust
slotmap::new_key_type! {
    pub struct CreatureId;
    pub struct ItemId;
}
```

This prevents mixing entity types:

```rust
let creature_id: CreatureId = world.creatures.insert(creature);
let item = world.items.get(creature_id); // COMPILE ERROR: type mismatch
```

## Removal and Invalidity

```rust
// Removing invalidates the key
world.creatures.remove(creature_id);

// Future accesses return None
assert!(world.creatures.get(creature_id).is_none());

// SlotMap may reuse the slot with a new generation
let new_id = world.creatures.insert(new_creature);
// new_id != creature_id (different generation)
```

## Optional References

Use `Option<CreatureId>` for optional targets:

```rust
pub struct Player {
    pub target: Option<CreatureId>,
    pub follow_target: Option<CreatureId>,
}

// Access pattern
if let Some(target_id) = player.target {
    if let Some(target) = world.creatures.get(target_id) {
        // Both exist
    }
}
```

## Performance Notes

- SlotMap access is O(1) with generational safety
- No allocations for ID storage (just a u64 internally)
- Cache-friendly: entities stored contiguously
- Iteration is fast: no indirection
