TFS Rust Port — Idiomatic Rust Audit
Scope: Code quality and Rust idioms only. No protocol changes. No feature changes.
Codebase snapshot: 2026-04-06
Audited crates: tfs-rust-common, tfs-rust-core, tfs-rust-net, tfs-rust-db, tfs-rust-content

Table of Contents
CreatureKind — Eliminate Repetitive Match Arms
Bitflags — Replace Manual Bitmask Enums
Error Handling — Strengthen the Type System
Traits — Extract Common Interfaces
Broadcast Boilerplate — DRY the Spectator Pattern
Type Safety — Newtypes and Stronger Enums
Naming & Module Conventions
Data Structures & Ownership
Testing & Debug Ergonomics
1. CreatureKind — Eliminate Repetitive Match Arms
IMPORTANT

This is the single highest-impact refactor. Every function that touches any creature field currently has a 3-arm match on CreatureKind to reach .base.

Current Pattern (repeated ~30+ times across the codebase)
Found in: 
kind.rs
, 
combat/mod.rs
, 
game_world.rs
, 
walk.rs

rust
// This pattern appears everywhere — 3 arms just to get `.base`
let base = match kind {
    CreatureKind::Player(p) => &mut p.base,
    CreatureKind::Monster(m) => &mut m.base,
    CreatureKind::Npc(n) => &mut n.base,
};
Proposed Fix: Add a base() accessor (you already have base_mut()!)
kind.rs:34-39
 already has base_mut(). Add the immutable version and use it everywhere:

rust
impl CreatureKind {
    pub fn base(&self) -> &CreatureBase {
        match self {
            CreatureKind::Player(p) => &p.base,
            CreatureKind::Monster(m) => &m.base,
            CreatureKind::Npc(n) => &n.base,
        }
    }
}
Then in 
combat/mod.rs:145-149
:

diff
-    let base = match kind {
-        CreatureKind::Player(p) => &mut p.base,
-        CreatureKind::Monster(m) => &mut m.base,
-        CreatureKind::Npc(n) => &mut n.base,
-    };
+    let base = kind.base_mut();
Same for apply_health_delta in 
combat/mod.rs:92-134
The entire function repeats the same health = clamp(health + delta, 0, max_health) logic 3 times. With base_mut():

rust
fn apply_health_delta(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    attacker: Option<CreatureId>,
    target: CreatureId,
    delta: i32,
) -> bool {
    let Some(kind) = creatures.get_mut(target) else { return false };
    let base = kind.base_mut();
    let old_hp = base.health;
    let new_hp = (old_hp + delta).clamp(0, base.max_health);
    if new_hp < old_hp {
        if let Some(aid) = attacker {
            *base.damage_map.entry(aid).or_insert(0) += (old_hp - new_hp) as u64;
        }
    }
    base.health = new_hp;
    old_hp != new_hp
}
Files affected: combat/mod.rs, game_world.rs (lines 397-427, can_see_creature_for_known_set), walk.rs (set_direction_from_step)

2. Bitflags — Replace Manual Bitmask Enums
Problem: ItemAttrFlags and CylinderFlags reinvent bitflags!
item_attributes.rs:54-141
 — The ItemAttrFlags enum with manual from_bits() is 87 lines of tedious match arms. The from_bits function silently maps any unknown bit combination to None, which is a correctness issue for combined flags.

rust
// Current: broken for combined flags
fn from_bits(bits: u32) -> Self {
    match bits {
        0 => Self::None,
        1 => Self::ActionId,
        // ... 30 more single-bit cases
        _ => Self::None, // BUG: ActionId | UniqueId → None
    }
}
Proposed: Use the bitflags crate
rust
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ItemAttrFlags: u32 {
        const ACTION_ID        = 1 << 0;
        const UNIQUE_ID        = 1 << 1;
        const DESCRIPTION      = 1 << 2;
        const TEXT             = 1 << 3;
        // ... all the rest
        const CUSTOM           = 1 << 31;
    }
}
This eliminates the entire from_bits function, the manual contains/insert/remove methods, and makes | / & / ! work correctly on combined flags.

Same for CylinderFlags
cylinder.rs:18-51
 — Currently a struct with bits: u32 and manual contains/union. The bitflags! macro gives you all of this plus BitOr, BitAnd, Debug formatting of flag names.

Same for tile::flags
tile.rs:8-13
 — Raw u32 constants in a module. Should be a proper bitflags type.

Same for walk.rs tile_state and cylinder flags
walk.rs:50-65
 — Duplicates tile flag constants as raw u32. These should reference a shared bitflags type from tile.rs.

3. Error Handling — Strengthen the Type System
3a. TfsRustError::Database(String) loses error context
error.rs:6
 — Every database error is stringified immediately. This kills the ability to pattern-match on specific SQLx errors (like deadlock codes) at higher layers.

rust
// Current: stringly-typed, pattern matching impossible
#[error("database error: {0}")]
Database(String),
// Proposed: preserve the original error
#[error("database error: {0}")]
Database(#[from] sqlx::Error),
NOTE

This requires sqlx to be a dependency of tfs-rust-common (or move the DB error variant to tfs-rust-db). If you want to keep error in common, use #[error(transparent)] Database(#[from] Box<dyn std::error::Error + Send + Sync>).

3b. .map_err(|e| TfsRustError::Database(e.to_string()))? is repeated ~20 times
player.rs
 — Lines 155, 225, 248, 278, 305, 321, 390, 396, 404, 427, 436, 441, 460. That exact .map_err closure appears 13 times in one file.

With #[from] sqlx::Error on the DB variant, all of these become just ?.

3c. ReturnValue exists twice
enums.rs:79-92
 has a truncated ReturnValue with ~10 variants + a comment "Truncated for brevity". 
return_value.rs
 has the full version with 76 variants. This is a duplicate type that will cause confusion. The enums.rs version should be deleted and everything should use the core one.

4. Traits — Extract Common Interfaces
4a. Creature trait for shared behaviour
Every creature type (Player, Monster, Npc) has a base: CreatureBase field and identical accessors. A trait eliminates the match + delegation pattern:

rust
pub trait Creature {
    fn base(&self) -> &CreatureBase;
    fn base_mut(&mut self) -> &mut CreatureBase;
    // Derived methods — free for all implementors
    fn position(&self) -> Position { self.base().position }
    fn is_summon(&self) -> bool { self.base().master.is_some() }
    fn has_condition(&self, ct: ConditionType) -> bool {
        self.base().active_conditions.iter().any(|c| c.ctype == ct)
    }
}
impl Creature for Player {
    fn base(&self) -> &CreatureBase { &self.base }
    fn base_mut(&mut self) -> &mut CreatureBase { &mut self.base }
}
// Same for Monster, Npc
4b. NetworkSerialize / Encodable trait for packet writing
item_encode.rs
 and 
creature_encode.rs
 have free functions like add_item_to_message(msg, ...). A trait would be more idiomatic:

rust
pub trait WriteToMessage {
    fn write_to(&self, msg: &mut NetworkMessage);
}
5. Broadcast Boilerplate — DRY the Spectator Pattern
Problem: Same "collect spectators → enqueue packet" loop repeated 6+ times
game_world.rs:349-357
, 
928-951
, 
955-978
, 
982-991
, plus 
walk.rs
 spectator loops.

rust
// This pattern appears 6+ times:
let conns: Vec<ConnId> = self.conn_to_creature.iter()
    .filter(|(_, &vid)| self.can_see_position(vid, pos))
    .map(|(&c, _)| c)
    .collect();
for conn in conns {
    let pkt = /* build packet */;
    self.enqueue_outgoing(conn, pkt.into_bytes());
}
Proposed: Extract a broadcast_to_spectators helper
rust
impl GameWorld {
    /// Sends a packet to all connections that can see `pos`.
    fn broadcast_to_spectators(&mut self, pos: Position, build: impl Fn() -> Vec<u8>) {
        let conns: Vec<ConnId> = self.conn_to_creature.iter()
            .filter(|(_, &vid)| self.can_see_position(vid, pos))
            .map(|(&c, _)| c)
            .collect();
        let pkt = build();
        for conn in conns {
            self.enqueue_outgoing(conn, pkt.clone());
        }
    }
}
Then broadcast_magic_effect, broadcast_tile_item_add, broadcast_tile_item_update, broadcast_tile_item_remove all collapse to 1-3 lines.

6. Type Safety — Newtypes and Stronger Enums
6a. Raw u8 / u16 for typed game constants
Location	Current	Proposed
game_world.rs:197
speak_type: u8	Use SpeakType enum
game_world.rs:332
broadcast_magic_effect(pos, 4)	Use MagicEffect::Poff
walk.rs:139
const MESSAGE_STATUS_SMALL: u8 = 21	Use a MessageType enum
creature/base.rs:15-21
look_type: i32	Use u16 (outfits are u16 on wire)
item.rs:25
item_type: u16	Newtype ServerItemId(u16)
6b. Outfit fields are i32 but the protocol uses u16
base.rs:14-21
 — All outfit fields are i32 (coming from C++ int). On the wire they're u16 or u8. This means every encode site needs a cast. Make them the correct size at the source.

6c. Direction::try_from(u8) is missing
enums.rs:1-11
 — Direction has repr(u8) discriminants but no TryFrom<u8> impl. The game parse code likely does raw matches instead of safe conversion. Add:

rust
impl TryFrom<u8> for Direction {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, ()> {
        match v {
            0 => Ok(Self::North),
            1 => Ok(Self::East),
            // ...
            _ => Err(()),
        }
    }
}
Same applies to CombatType, ConditionType, SkullType, SpeakType, MagicEffect, ShootEffect — all bare enums without wire conversion.

7. Naming & Module Conventions
7a. Wildcard re-exports pollute namespaces
common/lib.rs:12-18
:

rust
pub use enums::*;
pub use error::*;
pub use game_command::*;
pub use position::*;
pub use propstream::*;
pub use protocol_constants::*;
Six pub use * dumps every symbol from those modules into the tfs_rust_common namespace. Anyone using use tfs_rust_common::* gets hundreds of symbols. Replace with explicit re-exports of the key types:

rust
pub use enums::{Direction, CombatType, ConditionType, SkullType, ZoneType, ReturnValue};
pub use error::{TfsRustError, Result};
pub use position::Position;
// etc.
7b. Some file names don't follow Rust conventions
Current	Suggested
login_out.rs	login_packets.rs (describes what it does)
outgoing_extra.rs	Merge into outgoing.rs or rename to outgoing_protocol.rs
xtea_tfs.rs	Merge into xtea.rs (one module for one algorithm)
7c. ConfigManager getters should use impl AsRef<str> or typed keys
config.rs
 — All config access is get_string("mysqlHost") which is stringly-typed and typo-prone. An enum of config keys would catch errors at compile time:

rust
pub enum ConfigKey {
    MysqlHost,
    MysqlUser,
    ServerName,
    // ...
}
8. Data Structures & Ownership
8a. DashMap on a single-threaded game loop
game_world.rs:58-60
:

rust
pub player_by_name: DashMap<String, CreatureId>,
pub player_by_guid: DashMap<u32, CreatureId>,
The comment says GAME THREAD ONLY. If it's single-threaded, DashMap adds unnecessary overhead (sharding, atomic ops on every access). A plain HashMap is faster and makes the single-thread invariant obvious. If you need cross-thread access later, wrap the GameWorld or these maps in a proper synchronization primitive.

8b. Vec<ActiveCondition> should be SmallVec
base.rs:54
 — Most creatures have 0-3 active conditions. A SmallVec<[ActiveCondition; 4]> avoids heap allocation for the common case. (Requires the smallvec crate.)

8c. pending_outgoing: HashMap<ConnId, Vec<Vec<u8>>>
game_world.rs:72
 — Vec<Vec<u8>> is a Vec of heap allocations. Consider Vec<BytesMut> (you already depend on bytes) or a single BytesMut per connection with length-prefixed segments.

8d. Manual Clone for CreatureBase
base.rs:83-112
 — A 30-line manual Clone impl that exists only to set walk_timer: None. Instead, make walk_timer a separate non-cloneable field or wrap it in a type that implements Clone as no-op:

rust
#[derive(Default)]
pub struct WalkTimer(Option<tokio::task::JoinHandle<()>>);
impl Clone for WalkTimer {
    fn clone(&self) -> Self { Self(None) }
}
Then #[derive(Clone)] works on CreatureBase.

8e. ItemAttributes flat struct with 25+ fields
item_attributes.rs:177-215
 — Every item carries ~200 bytes of attributes even when most are zeroed. The bitmask tells you which are set, but the memory is still allocated. Consider a HashMap<ItemAttrFlags, AttrValue> or enum-based sparse storage for items that are uncommon. Most items (ground tiles) never use attributes.

TIP

If profiling shows memory is fine, this can wait. But for a server with 50k+ items loaded from OTBM, the difference is significant.

9. Testing & Debug Ergonomics
9a. Display impl missing on most game enums
Direction, CombatType, ConditionType, SkullType, ZoneType — none implement Display. Adding #[derive(strum::Display)] or manual impls makes log output readable:

// Current log:
tracing::debug!(?direction, "walk");
// Output: direction=NorthEast  (from Debug)
// With Display:
tracing::info!(%direction, "walk");
// Output: direction=north-east  (human readable)
9b. Container tests use ItemId::default()
container.rs:401-474
 — All tests create items with ItemId::default(), which means every item has the same ID. Tests that rely on contains() or index_of() may produce false positives. Use a SlotMap in tests to generate unique IDs.

9c. PropWriteStream — infallible write_* silently drops errors
propstream.rs:24-34
:

rust
pub fn write_u16(&mut self, v: u16) {
    let _ = self.buf.write_u16::<LittleEndian>(v);
}
Writing to a Vec<u8> via WriteBytesExt never fails (it's infallible on Vec), so the let _ = is harmless. But it's cleaner to use direct byte manipulation:

rust
pub fn write_u16(&mut self, v: u16) {
    self.buf.extend_from_slice(&v.to_le_bytes());
}
This removes the byteorder dependency for write-path code (reads still benefit from it).

Summary Priority Matrix
Priority	Category	Impact	Effort
🔴 High	1. CreatureKind base() accessor	Eliminates ~50 match arms	Low
🔴 High	2. bitflags! for ItemAttrFlags	Fixes combined-flag bug + -90 lines	Low
🔴 High	3c. Delete duplicate ReturnValue	Prevents type confusion	Trivial
🟡 Medium	3a-b. Error handling	Less boilerplate, better diagnostics	Medium
🟡 Medium	5. Broadcast helper	-60 lines, DRY	Low
🟡 Medium	6c. TryFrom for enums	Safer wire parsing	Low
🟡 Medium	8a. DashMap → HashMap	Removes unnecessary overhead	Low
🟡 Medium	8d. WalkTimer Clone wrapper	Removes 30 lines of manual Clone	Low
🟢 Low	4. Creature trait	Cleaner architecture	Medium
🟢 Low	6a-b. Newtypes	Compile-time correctness	Medium
🟢 Low	7a. Remove wildcard re-exports	Cleaner namespaces	Low
🟢 Low	8b. SmallVec for conditions	Micro-optimization	Trivial
🟢 Low	8e. Sparse ItemAttributes	Memory reduction	High
🟢 Low	9. Display, test IDs	Developer ergonomics	Low
TIP

Start with the 🔴 High items — they're low effort and high reward. Items 1, 2, and 3c can each be done in under 30 minutes.


Rust Idioms Audit — Execution Tasks
🔴 High Priority
[x] 1. Add CreatureKind::base() accessor and replace all manual 3-arm match destructures
[x] 2. Replace ItemAttrFlags manual bitmask with bitflags! crate
[x] 3. Delete duplicate ReturnValue from tfs-rust-common/src/enums.rs
🟡 Medium Priority
[x] 4. Extract broadcast_to_spectators helper to DRY broadcast loops
[x] 5. Replace DashMap with HashMap for game-thread-only lookups
[x] 6. Add WalkTimer clone wrapper to eliminate manual Clone impl
[x] 7. Add TryFrom<u8> for common wire enums (Direction, etc.)
🟢 Low Priority
[x] 8. Remove wildcard re-exports from tfs-rust-common/src/lib.rs
[x] 9. Add Display impls for game enums
[x] 10. PropWriteStream — use direct byte manipulation