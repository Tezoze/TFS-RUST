# Protocol Versioning — Making TFS Rust Version-Interchangeable

**Status:** Design / planning doc (no code written yet).
**Goal:** Support multiple Tibia protocol versions (and their mechanics) from one codebase, switchable by config.
**Anchors:** `7.72` (the `reference/tvp-772/gameserver/` C++ reference, "The Violet Project") and `10.98` (current Rust target, OTClientv8).
**Reference trees (each has a distinct role):**
- `reference/tvp-772/gameserver/src/` = **TVP**, a TFS fork ported to 7.72. → **Sole authority for 772 wire/packets**
  (§2–§11): opcodes, byte layouts, login, transport. **Never** use `reference/classic-772/tibia-game-master` or repo-root
  `src/` for 772 protocol work.
- `reference/classic-772/tibia-game-master/src/` = the leaked 772 **decompile**. → **772 behavioral source of truth**
  (§12 mechanics only — not wire). When TVP and the decompile disagree on a *game outcome*, the
  decompile wins; wire always follows `gameserver/src/`.
- repo-root `src/` = TFS 1.4.2 / 10.98 = what the Rust port currently mirrors.

**Guiding principles (apply throughout):**
1. **Outcome parity, not code parity.** C++ reference defines *observable behavior* (bytes, damage, ticks, DB results). Implement in idiomatic Rust (`SlotMap`, enums, traits, channels) — never line-translate or transcribe reference source.
2. **Clean-room behavior, not code.** Copy the *observable outcomes* of the decompile (damage rolls,
   step timing, AI decisions), validated against captured results — **never transcribe its source**.
   Implement in our own Rust, in TFS/TVP style.
3. **Stay TFS-style.** Keep the easy, Lua-first scripting surface and TVP's structure; do not invent a
   foreign architecture.
4. **No magic numbers.** Combat/damage/speed/attack-speed/exp/condition formulas are **data- and
   Lua-tunable** (§12.13), not constants buried in Rust. Anyone should be able to retune a shard
   without recompiling.

---

## 1. Why this is needed

Today the Rust port is **hardcoded to 10.98** at every layer of `tfs-rust-net`. There is no version
concept in config or in code. Concrete evidence:

- Type/function names bake the version in: `PlayerStats1098`, `send_player_stats_1098`,
  `send_player_skills_1098` (`crates/tfs-rust-net/src/outgoing_extra.rs`).
- A single global opcode table with 10.x values only:
  `crates/tfs-rust-common/src/protocol_opcodes.rs` (e.g. `MARKET_*`, `TOGGLE_MOUNT = 0xD4`,
  `EQUIP_OBJECT = 0x77`, shop `0x79–0x7C`).
- Item/creature/outfit encoders assume 10.x layout:
  - `item_encode.rs` writes the `0xFF` MARK byte, animation `0xFE`, OTCv8 description string,
    duration byte — none of which exist in 7.72.
  - `creature_encode.rs` writes creature-type byte, guild emblem, speech bubble, MARK, helpers
    `u16`, walkthrough byte, mount `u16` — none exist in 7.72.
- Transport assumes Adler32 + pre-login challenge always on:
  `protocol_game.rs::decrypt_xtea_game_body` validates an Adler32 header unconditionally;
  `server.rs::handle_game_connection` always sends a `0x1F` challenge first. 7.72 does **neither**.
- Login assumes account **name** string + session key; 7.72 uses account **number** (`u32`) and
  inline credentials with no session key.

So "supporting 7.72" is not a config toggle today — it requires introducing a version abstraction
first. This doc defines that abstraction and the migration steps.

---

## 2. The two protocols at a glance (7.72 vs 10.98)

All 7.72 line references are in `gameserver/src/`. All 10.98 references are in repo-root `src/`
and the existing Rust files.

### 2.1 Transport / framing

| Concern | 7.72 (`gameserver/`) | 10.98 (current Rust) |
|---|---|---|
| TCP frame | `u16` LE body size + body | same |
| Adler32 checksum header | **absent** (never validated; `adlerChecksum` is dead code in `tools.cpp`) | **present**, 4 bytes, validated on recv |
| `INITIAL_BUFFER_POSITION` | 4 | 8 |
| XTEA recv slack | `length - 4` | `length - 6` |
| Pre-login challenge (`0x1F`) | **none** (`onConnect` not implemented) | server sends first |
| Compression (zlib) | none | optional |
| RSA | 128-byte block, leading 0 byte | same |
| XTEA delta | `0x9E3779B9` | same |

Rust touch points: `protocol_game.rs` (`decrypt_xtea_game_body`, `encrypt_xtea_game_frame`),
`game_first_packet.rs`, `server.rs` (`handle_game_connection` challenge), `adler.rs`,
`game_challenge.rs`.

### 2.2 Login flow

| Concern | 7.72 | 10.98 |
|---|---|---|
| Account identity | **`u32` account number** | account **name** string |
| Login port packet id | first body byte `0x01` | (modern flow) |
| Game port packet id | first body byte `0x0A` | `0x0A` prelude too, but session-key based |
| Credentials on game port | GM flag `u8` + account `u32` + char name + password, inline | session key string (`acc\npass\ntoken\ntime`) + char name |
| 2FA token | none | optional second RSA block |
| Session key packet `0x28` | absent | present |
| Char list entry | name + world name + `u32` IP + `u16` port | world table + online flags |
| Premium | `u16` days | `u8` flag + `u32` unix timestamp |
| Self-appear opcode | **`0x0A`** (id + `u16` beat + `u8` canReportBugs) | **`0x17`** + speed doubles + store URL + enter-world chain |

Rust touch points: `game_first_packet.rs` (`parse_login_first`, `parse_game_first`),
`protocol_login_out.rs`, `pending_login.rs`, `server.rs` (both handlers),
`crates/tfs-rust-db` auth (`loginserver_authentication`, `gameworld_authentication` — by id vs name).

### 2.3 Item serialization (`NetworkMessage::addItem`)

7.72 (`gameserver/src/networkmessage.cpp` ~L82–106):

```
u16 clientId
[u8 count]          // if stackable
[u8 liquidColor]    // if splash/fluid (getLiquidColor)
```

Minimum **2 bytes**, no MARK, no animation phase, no description, no duration.

10.98 (current `item_encode.rs`): `u16 clientId` + `0xFF` MARK + count/fluid + optional `0xFE`
animation + optional OTCv8 description string + duration byte(s).

> Note: 7.72 fluid uses `getLiquidColor()` directly, **not** the `fluidMap` table used by 10.x.
> Verify the mapping in `gameserver/src/tools.cpp` before reusing `FLUID_MAP`.

### 2.4 Creature serialization (`AddCreature`)

| Field | 7.72 (`protocolgame.cpp` ~L2051) | 10.98 (`creature_encode.rs`) |
|---|---|---|
| Known header | `u16 0x62` + `u32 id` | same |
| Unknown header | `u16 0x61` + `u32 removeId` + `u32 id` + **name** | + **creature-type `u8`** before name |
| Health % | `u8` | `u8` |
| Direction | `u8` | `u8` |
| Outfit | see below | see below |
| Light level/color | `u8` + `u8` | `u8` + `u8` |
| Speed | **`u16` full `getStepSpeed()`** | `u16` `getStepSpeed()/2` |
| Skull / party shield | `u8` + `u8` | `u8` + `u8` |
| Guild emblem | **absent** | `u8` (unknown only) |
| Creature-type (2nd) | **absent** | `u8` |
| Speech bubble | **absent** | `u8` |
| MARK `0xFF` | **absent** | `u8` |
| Helpers | **absent** | `u16` |
| Walkthrough byte | **absent** | `u8` |

### 2.5 Outfit (`AddOutfit`)

| Field | 7.72 | 10.98 |
|---|---|---|
| lookType | `u16` | `u16` |
| if `lookType != 0` | head/body/legs/feet (`u8`×4) | + **addons `u8`** |
| if `lookType == 0` | `u16 lookTypeEx` | same |
| Mount | **absent** | `u16 lookMount` trailing |

Client `setOutfit` in 7.72 sends only `u16 lookType` + 4 color bytes (no addons, no mount).

### 2.6 Player stats / skills

| Field | 7.72 (`AddPlayerStats` ~L2090) | 10.98 (`PlayerStats1098`) |
|---|---|---|
| Health/max | `u16`/`u16` | `u16`/`u16` |
| Capacity | **`u16`** (`freeCapacity/100`) | `u32` free + `u32` total |
| Experience | **`u32`** | **`u64`** |
| Level + % | `u16` + `u8` | same |
| XP-rate / stamina block | **absent** | several `u16` |
| Mana/max | `u16`/`u16` | `u16`/`u16` |
| Magic level | `u8` + `u8`% | `u8` + `u8` base + `u8`% |
| Soul | `u8` | `u8` |

Skills (`AddPlayerSkills`): 7.72 = 7 skills × (`u8` level + `u8`%). 10.98 = per skill `u16` level +
`u16` base + `u8`% plus a special-skills block.

Condition icons (`sendIcons` 0xA2): 7.72 = `u8`, 10.98 = `u16`.

### 2.7 Opcodes

7.72 lacks the modern blocks entirely: shop (`0x79–0x7C`), market (`0xF4–0xF8`), quest log
(`0xF0–0xF1`), mount toggle (`0xD4`), equip (`0x77`), wrap (`0x8B`), seek/browse container
(`0xCB–0xCC`), VIP edit (`0xDE`). Rule-violation reports use `0x9B–0x9D` in 7.72 vs `0xF2` in 10.x.
Many shared opcodes keep the same byte value but a **different payload shape** (containers, trade,
self-appear).

### 2.8 Containers / trade

- 7.72 `sendContainer` (`0x6E`): cid + item + name + `u8` capacity + `u8` hasParent + `u8` count +
  items. No unlock flag, no pagination, no `u16` size, no firstIndex (all 10.x additions).
- 7.72 `sendAddContainerItem` (`0x70`): cid + item, **no slot index** (10.x adds `u16` slot).
- Container slot updates use `u8` slot in 7.72 (10.x uses `u16`).
- No wire-level shop window in 7.72 — NPC commerce is script-driven.

---

## 3. What is shared vs version-specific

The good news: the protocol is a **thin shell around a version-agnostic core**. Almost all of
`tfs-rust-core`, `tfs-rust-content`, `tfs-rust-db`, the Lua layer, and even the crypto/buffer
primitives in `tfs-rust-net` are shared across versions unchanged. Only the **wire encoding/decoding
boundary** is version-specific.

> **Scope of this section = wire format.** "Shared" here means *no codec/byte work* — the modules
> below never emit version-specific bytes. A separate axis, **game-mechanics behavior** (combat
> timing, pathing cost, monster strategy, condition ticks), *does* differ between 7.72 and 10.98 even
> though it touches no wire bytes. That axis is covered in §12 and gated by a `MechanicsProfile`, not
> the wire codec. So a module can be "shared" for the codec yet still need an era-specific constant.

### 3.1 Layering

```
┌──────────────────────────────────────────────────────────────┐
│ SHARED (version-agnostic)                                      │
│  tfs-rust-core   game simulation, mechanics, world state       │
│  tfs-rust-content OTB/OTBM/XML loaders (code), item DB model    │
│  tfs-rust-db     schema, queries (except auth identity/charlist)│
│  tfs-rust-lua    scripting API                                  │
│  tfs-rust-common Position, ids, NetworkMessage buffer, enums    │
│  net primitives  RSA, XTEA core, Adler32 algorithm, message.rs  │
└───────────────────────────────┬──────────────────────────────┘
                                 │  neutral wire structs (ItemWire, AddCreatureWire, …)
                  ┌──────────────┴───────────────┐
                  ▼ Codec772                       ▼ Codec1098 (+OTCv8)
        VERSION-SPECIFIC: byte layout, opcodes, framing flags, login shape
```

### 3.2 Fully shared — no *wire/codec* changes needed

These operate on **game state**, never on client bytes. They are correct for any version as long as
the codec translates their effects to the wire. (Some also have era-specific *behavior* — e.g. walk
timing, combat, AI — which is a separate concern tracked in §12, not a codec change.)

| Area | Files (examples) |
|---|---|
| Walk physics, speed formula, auto-walk pathing | `walk.rs`, `walk_action.rs`, `pathfinding.rs` |
| Monster/NPC AI & thinking | `monster_ai.rs`, `creature_think.rs`, `monster_distance_step.rs`, `creature/npc.rs`, `creature/monster.rs` |
| Combat math, RNG, PvP rules, death/loot | `combat/mod.rs`, `combat/rng.rs`, `combat/pvp.rs`, `death.rs`, `weapon.rs` |
| Conditions, decay, spawns | `condition.rs`, `decay.rs`, `spawn.rs`, `spawn_lifecycle.rs` |
| Cylinder model, containers, inventory rules | `cylinder.rs`, `container.rs`, `container_ops.rs`, `inventory.rs`, `player_inventory_query_add.rs` |
| Items runtime model & attributes | `item.rs`, `item_attributes.rs`, `item_constants.rs`, `item_look.rs` |
| Map data, tiles, LOS, spectator quadtree, light | `map/mod.rs`, `tile.rs`, `map/los.rs`, `map/qtree.rs`, `world_light.rs`, `creature/light.rs` |
| Party, guild, house, vocation, spells | `party.rs`, `guild.rs`, `house.rs`, `creature/vocation.rs`, `spell.rs` |
| Scheduler, ids, return values, wildcard, scope | `scheduler.rs`, `ids.rs`, `return_value.rs`, `wildcard.rs`, `lua_scope.rs` |
| Net primitives (algorithm, not framing policy) | `rsa.rs`, `xtea.rs`, `xtea_tfs.rs`, `adler.rs`, `message.rs` |
| Map description **skip-RLE algorithm** | `map_description.rs` (the tile/creature *payloads* differ; the skip loop + viewport `8×6` are identical 7.72↔10.98) |

> The map viewport (`MAX_CLIENT_VIEWPORT_X = 8`, `MAX_CLIENT_VIEWPORT_Y = 6`) and the
> ground→items→creatures→down-items tile stack order are the **same** in 7.72 and 10.98, so the
> outer `GetMapDescription` / `GetFloorDescription` loop is shared. Only `write_item` / `write_creature`
> inside it are codec calls.

### 3.3 Shared code, version-bound *data* (not a code-fork)

| Area | Note |
|---|---|
| `tfs-rust-content` loaders | The OTB/OTBM/`items.xml` parsing **code** is shared; the **data files** (client item ids, `.spr`/`.dat` signatures, OTBM item ids) must match the chosen client. A 7.72 server needs 7.72 assets — that is data selection, not a codec. |
| Lua scripts (`data/`) | API is shared; individual scripts may assume version-specific item/outfit ids. Out of scope for the wire layer. |

### 3.4 Shared mechanics, version-specific *persistence* — confirm, don't assume

| Area | Note |
|---|---|
| `item_blob.rs` (`write_item_blob`/`parse_item_blob`) | This is the **database** serialization format, not the client wire format. It is shared and tied to the schema, **not** the protocol version. Do **not** route it through the codec. |
| `tfs-rust-db` queries | Schema-compatible and shared **except**: account identity (number vs name), the character-list query/shape, and premium representation (`u16` days vs `u32` timestamp). Only those auth/list paths need a version branch. |

### 3.5 The coupling that must be fixed (shared logic, hardcoded emission)

The mechanics are shared, but today the game core **emits 10.98 bytes inline** by calling
`tfs-rust-net`'s 10.98 builders directly. These call sites are the work — not the mechanics
themselves. They must instead fill **neutral wire structs** and hand them to the connection's codec:

| Core file | Hardcoded net call(s) today |
|---|---|
| `game_world.rs` | `send_player_stats_1098` / `PlayerStats1098`, `send_add/update_tile_item_template` |
| `login_out.rs` | `AddCreatureWire`/`OutfitWire`, `send_player_skills_1098` |
| `walk.rs` | `send_map_description_packet`, `send_move_creature_*`, `send_creature_turn` |
| `container_ui.rs` | `write_item_template` |
| `game_world_inventory.rs` | `send_inventory_item_template` |
| `player_inventory_notifications.rs` | `send_creature_light` |
| `spawn_lifecycle.rs` | `check_creature_known`, creature add/remove sends |
| `player_ping.rs` | `send_ping` / `send_ping_back` |

**Implication for the design in §4:** the game thread must hold (or look up) a `Codec` per
connection and build packets through it, instead of importing `*_1098` functions. Once that
indirection exists, adding `Codec772` requires **zero** changes to the mechanics above.

---

## 4. Target architecture

The core idea: **stop hardcoding 10.98** and route every version-sensitive read/write through a
version-aware boundary. Three coordinated mechanisms:

### 4.1 A `ProtocolVersion` value (single source of truth)

Add to `tfs-rust-common`:

```rust
// crates/tfs-rust-common/src/protocol_version.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolVersion(pub u16); // 772, 1098, ...

impl ProtocolVersion {
    pub const V772: Self = Self(772);
    pub const V1098: Self = Self(1098);
}
```

Derive a **capability set** from it instead of scattering `if version >= X` checks:

```rust
#[derive(Debug, Clone, Copy)]
pub struct ProtocolCaps {
    pub adler_checksum: bool,      // 7.72: false, 10.98: true
    pub prelogin_challenge: bool,  // 7.72: false
    pub account_name_login: bool,  // 7.72: false (uses account number)
    pub session_key_login: bool,   // 7.72: false
    pub item_mark_byte: bool,      // 0xFF MARK
    pub item_animation_byte: bool,
    pub creature_type_byte: bool,
    pub outfit_addons: bool,
    pub outfit_mount: bool,
    pub speed_halved: bool,        // 7.72: false
    pub stats_u64_experience: bool,
    pub skills_u16: bool,
    pub icons_u16: bool,
    pub self_appear_opcode: u8,    // 7.72: 0x0A, 10.98: 0x17
    // ...
}

impl ProtocolCaps {
    pub fn for_version(v: ProtocolVersion) -> Self { /* table lookup */ }
}
```

> A capability struct (one `match` at startup) is preferred over `version >= N` checks sprinkled
> through the encoders. It keeps each version's truth in one place, is cheap to copy, and makes the
> "what differs" matrix in §2 executable. This mirrors how OTClient gates behavior on
> `GameFeature` flags — see `docs/OTCLIENT_INFO.md`.

### 4.2 A `ProtocolCodec` trait (the seam)

Encoders/decoders become methods behind a trait, with one impl per version family. This is the
idiomatic Rust replacement for the C++ "one `ProtocolGame` recompiled per version" model — zero
per-call branching where layouts are picked once via a `&dyn ProtocolCodec` (or an enum dispatcher
for zero-cost static dispatch).

```rust
// crates/tfs-rust-net/src/codec/mod.rs
pub trait ProtocolCodec {
    fn caps(&self) -> ProtocolCaps;

    // incoming
    fn parse_game_opcode(&self, op: u8, msg: &mut NetworkMessage) -> Result<GamePacket>;
    fn decrypt_game_body<'a>(&self, body: &'a mut [u8], keys: &RoundKeys) -> Result<&'a [u8]>;

    // outgoing
    fn write_item(&self, msg: &mut NetworkMessage, item: &ItemWire);
    fn write_creature(&self, msg: &mut NetworkMessage, c: &AddCreatureWire);
    fn write_outfit(&self, msg: &mut NetworkMessage, o: &OutfitWire);
    fn write_player_stats(&self, msg: &mut NetworkMessage, s: &PlayerStatsWire);
    fn write_player_skills(&self, msg: &mut NetworkMessage, s: &PlayerSkillsWire);
    // ...
}

pub struct Codec772;
pub struct Codec1098;
impl ProtocolCodec for Codec772 { /* 7.72 layouts */ }
impl ProtocolCodec for Codec1098 { /* current behavior, renamed */ }
```

Prefer an **enum dispatcher** over `dyn` for the hot path to keep it zero-cost:

```rust
pub enum Codec { V772(Codec772), V1098(Codec1098) }
// delegate each method with a match; the branch predicts perfectly and inlines.
```

Key design rules:

- The encoders take **version-neutral input structs** (`ItemWire`, `AddCreatureWire`,
  `PlayerStatsWire`, …) carrying *all* fields. Each codec writes only the subset its wire format
  needs. The game thread never has to know the version.
- Opcodes become **per-version tables**, not one global module (see §4.3).
- The `NetworkMessage` primitive (`message.rs`) stays version-agnostic — it only does
  `write_u8/u16/u32/u64/string`. Good as-is.

### 4.3 Per-version opcode tables

Split `protocol_opcodes.rs` into a trait or per-version maps. Two viable shapes:

1. **Const tables per version** (`opcodes::v772`, `opcodes::v1098`) plus a small reverse map for
   outgoing. Decoders match on the table belonging to the connection's codec.
2. **A semantic enum** (`ClientOp::Move(Direction)`, `ClientOp::Attack`, …) with
   `fn from_byte(op: u8, v: ProtocolVersion) -> Option<ClientOp>` and
   `fn to_byte(self, v: ProtocolVersion) -> u8`. This is the cleanest: the rest of the codebase
   speaks semantic ops and never sees raw bytes.

Recommended: option 2 for client→server (already half-done — `GamePacket` is semantic), and
per-version `send` opcode constants for server→client.

---

## 5. Config plumbing

Add a single typed setting, defaulting to the current behavior so nothing breaks.

1. `config.lua.dist`: add `clientVersion = 1098` (or `protocolVersion`).
2. `crates/tfs-rust-core/src/config.rs`: add `pub protocol_version: u16` to the relevant config
   struct (alongside `NetConfig`), read via the existing `get_i64_or(cfg, "clientVersion", 1098)`
   helper. Validate against a known set (`772`, `1098`) and error early on unsupported values.
3. Thread the resolved `ProtocolVersion` into `GameWireConfig` / `LoginWireConfig`
   (`crates/tfs-rust-net/src/server.rs`) so each connection builds the right `Codec`.
4. Load `data/formulas/<clientVersion>.lua` into `MechanicsProfile` at startup (Track B, §12.13).
5. Store the `Codec` (or just `ProtocolVersion` + `ProtocolCaps`) per connection so the game thread
   can pick the right encoder when flushing output.

Env override (consistent with existing `TFS_*` overrides): `TFS_PROTOCOL_VERSION`.

---

## 6. Migration plan (phased, low-risk)

There are **two parallel tracks**: **Track A (wire)** makes byte output version-selectable, and
**Track B (mechanics)** makes 7.72 *behavior* selectable using the 772 decompile
(`tibia-game-master/src/`) as the source of truth (§12). Track A is required to connect a 7.72
client; Track B is required for the shard to *behave* like 7.72. They can proceed independently —
A delivers a connectable-but-10.98-behaving 7.72 client; B can be validated on the existing 10.98
shard before the wire flips.

Each phase compiles and keeps 10.98 working. Do not attempt 7.72 wire output until Phase A5, and do
not flip mechanics defaults until Phase B5.

### Track A — Wire / protocol

#### Phase A0 — Scaffolding (no behavior change)
- Add `ProtocolVersion`, `ProtocolCaps`, and `ProtocolCaps::for_version` in `tfs-rust-common`.
- Add the `protocol_version` config key (default `1098`) and plumb it to `*WireConfig`. Unused for now.
- Tests: `ProtocolCaps::for_version(1098)` matches today's hardcoded assumptions.

#### Phase A1 — Introduce the codec seam (10.98 only)
- Create `codec/mod.rs` with the `ProtocolCodec` trait + `Codec1098`.
- Move (not rewrite) existing encoders behind `Codec1098`: rename
  `PlayerStats1098`→`PlayerStatsWire` (neutral), keep the 10.98 byte writer as `Codec1098`'s impl.
- Replace direct calls in the game/output path with calls through the per-connection codec.
- Golden-byte tests (`tests/protocol_compat.rs`) must produce **identical bytes** to before.

#### Phase A2 — Per-version opcodes
- Introduce semantic client-op decoding keyed by version (or v1098 table) and per-version server
  send-opcode constants. 10.98 values unchanged.

#### Phase A3 — Transport capability gating
- Make Adler32 (`decrypt_xtea_game_body` / `encrypt_xtea_game_frame`), the pre-login challenge
  (`server.rs`), and the XTEA slack (`-4` vs `-6`) honor `ProtocolCaps`. Defaults keep 10.98 intact.

#### Phase A4 — Login capability gating
- Branch `game_first_packet.rs` and `protocol_login_out.rs` on `account_name_login` /
  `session_key_login`. Add account-number auth path in `tfs-rust-db`. Gate self-appear opcode
  (`0x0A` vs `0x17`).

#### Phase A5 — Implement `Codec772`
- Item/creature/outfit/stats/skills/containers/trade per §2. Verify fluid color path
  (`getLiquidColor` vs `FLUID_MAP`).
- Add golden-byte tests captured from `gameserver/` behavior (see §7).

#### Phase A6 — Wire it up & document deviations
- `clientVersion = 772` selects `Codec772` end to end. Smoke test with a 7.72 client.
- Update `docs/PROJECT_STATUS.md`, `tasks/lessons.md`, and module C++ refs (cite `gameserver/src/...`).

### Track B — Mechanics (CipSoft `tibia-game-master/src/` as truth)

Each phase: read the cited CipSoft files, extract the **outcomes** (constants/formulas, §12),
re-implement them in TVP/TFS style behind a Lua-loaded `MechanicsProfile` (§12.11, §12.13) defaulting
to today's 10.98 values, and cite **both** `tibia-game-master/src` (behavior) and `gameserver/src`
(style) in module headers (per `.cursor/rules/TFS-cpp-references.mdc`). **Clean-room:** copy outcomes,
not code (R12). **No magic numbers:** every extracted constant becomes a `MechanicsProfile` /
`data/formulas/<version>.lua` value (R11). Behavior stays 10.98 until Phase B5 flips the 772 profile.

#### Phase B0 — `MechanicsProfile` + Lua loader (no behavior change)
- Add a `MechanicsProfile` value (mirror of `ProtocolCaps`) carrying the §12.11 knobs: beat ms, path
  cost model, attack-speed source, armor-reduction mode, fight-mode modifiers, weakest-target metric,
  distance-keep range, damage formula, condition-tick constants, spawn-near-player policy.
- Load it from a **version-specific** Lua file (Tier-1 constants): `data/formulas/772.lua` or
  `data/formulas/1098.lua`, selected by `clientVersion`. Each file holds that era's defaults; add the
  optional Tier-2 formula-override hooks (§12.13) — registered but no-op by default.
- Thread it onto the game thread alongside `ProtocolVersion`. Unit-test that the loaded 1098 values
  match today's constants and that a missing `data/formulas/<version>.lua` falls back to built-in
  `for_version` defaults.

#### Phase B1 — Movement & scheduling (`cract.cc`, `crmain.cc`)
- Audit-map: `TCreature::Execute`/`CalculateDelay`/`NotifyGo` (`cract.cc`), `GetSpeed` (`crmain.cc:445`).
- Make beat quantization (200 vs 50 ms) and the step-delay formula read from `MechanicsProfile`.
- Rust touch points: `walk.rs`, `creature_think.rs`. Keep TFS speed formula; profile only the quantizer.

#### Phase B2 — Pathfinding (`cract.cc` `TShortway`)
- Audit-map: reverse-A\* with terrain-weighted `WAYPOINTS` cost, diagonal 3× (`cract.cc:7–262`).
- Add a 772 cost model option to `get_path_matching` (`pathfinding.rs`); 1098 keeps fixed 10/25.

#### Phase B3 — Monster AI (`crnonpl.cc`, `script.cc`)
- Audit-map: `IdleStimulus`/`Strategy[4]`/`IsFleeing`/distance-4 (`crnonpl.cc`).
- Profile the weakest-target metric (current vs max HP), distance-keep range, lose-target roll, and
  spawn-near-player policy in `monster_ai.rs` / `spawn_lifecycle.rs`.
- **NPCs (772 and 1098):** stay **TFS-faithful Lua-only** — `data/npc/scripts/` + npcsystem, same as
  repo-root TFS 1.4.2. Do **not** port TVP's native `.ndb` engine (`npcbehavior.cpp`). For a 772 shard,
  convert TVP `gameserver/data/npc/behavior/*.ndb` → Lua scripts out-of-band (content migration, §12.6).

#### Phase B4 — Combat, skills, conditions, magic (`crcombat.cc`, `crskill.cc`, `magic.cc`)
- Audit-map: attack cooldown 2000 ms, `ProbeValue` damage, randomized armor, fight-mode %, exp
  distribution (`crcombat.cc`); skill/level curves (`crskill.cc`); DoT timer-skills (`crskill.cc`);
  spell multiplier `2*lvl+3*ml` (`magic.cc`).
- Implement on top of the combat skeleton (`combat/mod.rs`) and Phase G condition ticks
  (`condition.rs`); route every era-divergent constant through `MechanicsProfile`, and expose the
  `getWeaponDamage` / `getArmorReduction` / `getAttackSpeed` / `getSpellDamage` / `getConditionTick`
  Lua override hooks (§12.13). Validate outputs against captured CipSoft values (clean-room, R12).

#### Phase B5 — Flip & validate the 772 profile
- `clientVersion = 772` loads `data/formulas/772.lua` into the 772 `MechanicsProfile`. Validate step
  cadence, monster behavior,
  combat numbers, and DoT ticks against CipSoft constants.
- Capture lessons in `tasks/lessons.md`; document any CipSoft↔TFS deviations chosen.

---

## 7. Testing strategy

- **Golden bytes per version.** Extend `crates/tfs-rust-net/tests/protocol_compat.rs` into
  per-version modules. For each version, assert exact byte output for: item (stackable / fluid /
  plain), creature (known / unknown), outfit (looktype / item-outfit), stats, skills, map
  description, self-appear, container open.
- **Capability invariants.** Unit-test `ProtocolCaps::for_version` so the §2 matrix is enforced in
  code (e.g. `assert!(!caps772.adler_checksum)`).
- **Round-trip.** XTEA frame encode→decode under each transport profile (with/without Adler32).
- **Reference capture.** Where exact bytes are uncertain, capture real frames from the 7.72
  `gameserver/` (proxy via `tools/packet-proxy`) and freeze them as fixtures. Per project rules,
  do not guess wire layout — confirm against `gameserver/src/` and cite file + function.

---

## 8. Files to touch (checklist)

| File | Change |
|---|---|
| `crates/tfs-rust-common/src/protocol_version.rs` (new) | `ProtocolVersion`, `ProtocolCaps` |
| `crates/tfs-rust-common/src/protocol_opcodes.rs` | per-version tables / semantic ops |
| `crates/tfs-rust-common/src/lib.rs` | export new module |
| `crates/tfs-rust-net/src/codec/mod.rs` (new) | `ProtocolCodec`, `Codec` enum |
| `crates/tfs-rust-net/src/codec/v1098.rs` (new) | move current encoders here |
| `crates/tfs-rust-net/src/codec/v772.rs` (new) | 7.72 encoders (Phase A5) |
| `crates/tfs-rust-net/src/item_encode.rs` | become `Codec` methods / neutral `ItemWire` |
| `crates/tfs-rust-net/src/creature_encode.rs` | gate fields by caps |
| `crates/tfs-rust-net/src/outgoing_extra.rs` | rename `*1098` → neutral, move byte writer into codec |
| `crates/tfs-rust-net/src/protocol_game.rs` | Adler/XTEA slack via caps |
| `crates/tfs-rust-net/src/game_first_packet.rs` | login layout via caps |
| `crates/tfs-rust-net/src/protocol_login_out.rs` | char list / premium per version |
| `crates/tfs-rust-net/src/server.rs` | challenge gating; carry `Codec` per conn |
| `crates/tfs-rust-net/src/game_parse.rs` | version-keyed opcode dispatch |
| `crates/tfs-rust-net/src/map_description.rs` | item/creature writes via codec |
| `crates/tfs-rust-core/src/config.rs` | `protocol_version` key + validation |
| `crates/tfs-rust-db/src/...` (auth) | account-number login path for 7.72 |
| `config.lua.dist` | `clientVersion = 1098` |
| `crates/tfs-rust-net/tests/protocol_compat.rs` | per-version golden tests |

**Core emission call sites (rewire to codec, §3.5 — mechanics unchanged):** `game_world.rs`,
`login_out.rs`, `walk.rs`, `container_ui.rs`, `game_world_inventory.rs`,
`player_inventory_notifications.rs`, `spawn_lifecycle.rs`, `player_ping.rs`. These replace direct
`*_1098` / `*_template` imports with calls through the per-connection `Codec`. **Do not touch** the
shared mechanics modules listed in §3.2, nor `item_blob.rs` (DB format, §3.4).

**Mechanics track (§12, Track B) — additional files:**

| File | Change |
|---|---|
| `crates/tfs-rust-core/src/formulas.rs` (new) | `MechanicsProfile` + version-keyed Lua loader + Tier-2 hook registry (§12.13) |
| `data/formulas/772.lua` (new) | 7.72 / 772-faithful tunable constants + optional formula overrides |
| `data/formulas/1098.lua` (new) | 10.98 / TFS 1.4.2 tunable constants + optional formula overrides |
| `crates/tfs-rust-lua/...` | bind `getWeaponDamage`/`getStepDuration`/… override hooks |
| `crates/tfs-rust-core/src/{walk,combat/mod,condition,spell,monster_ai,spawn_lifecycle}.rs` | read constants/formulas from `MechanicsProfile`; remove balance literals (R11) |
| `data/npc/scripts/` (772 content) | convert TVP `.ndb` NPCs → TFS Lua scripts (§12.6); no Rust `.ndb` engine |

---

## 9. Adopt now — low-cost changes while building on 1098

You do **not** need the full codec split yet. But every new feature added with raw 1098 bytes makes
the eventual 7.72 work bigger. These are cheap habits/changes to adopt **now** so future-you isn't
unpicking hardcoded assumptions. None of them change current behavior.

1. **Stop baking the version into names.** Rename `PlayerStats1098` → `PlayerStatsWire`,
   `send_player_stats_1098` → `encode_player_stats` (`outgoing_extra.rs`). New encoders get
   neutral names from the start. This alone removes most future rename churn.
2. **Add the `ProtocolVersion` + `ProtocolCaps` value now (Phase A0).** Even unused, thread it through
   `GameWireConfig` / `LoginWireConfig` and store it per connection. The plumbing is the tedious part;
   doing it early means new code can read `caps` instead of assuming.
3. **All byte-building stays in `tfs-rust-net`.** Core must never call `msg.write_u8(opcode)` or hand-roll
   wire bytes. Today `container_ui.rs` calls `write_item_template` (a net builder) — that's fine; the rule
   is: byte layout knowledge lives only in net. Keep it that way for every new packet.
4. **New outgoing packets take a neutral input struct.** Define `XxxWire { all fields }` + one encoder
   function, rather than an encoder that takes already-narrowed primitives. This is the exact shape the
   codec seam needs later, at no extra cost now.
5. **Carry full-width values across the boundary; narrow in the encoder.** Wire structs should hold the
   widest representation (e.g. `experience: u64`, `free_capacity: u32`). The 1098 encoder writes them
   wide; a future 772 encoder narrows (`u32` exp, `u16` cap). Never pre-narrow in core.
6. **Centralize opcodes.** Never inline opcode bytes; always reference
   `tfs-rust-common/src/protocol_opcodes.rs`. Add every new opcode there. `game_parse.rs` already does
   this — keep new code consistent.
7. **Keep OTCv8 quirks flagged, not blended.** Mark OTCv8-only behavior (omitted duration byte, 6-vs-7
   special-skill rows, description string) with a clear comment/flag so it is not mistaken for generic
   "1098 truth" later. See `docs/OTCLIENT_INFO.md`.
8. **Tag version-specific code so it's greppable.** Put a marker comment on any function/field/branch whose
   layout is protocol-specific, e.g. `// PROTOCOL: 1098 wire layout`. A single grep then yields the full
   surface that `Codec772` must cover.
9. **Capture the 772 C++ ref when you're already in the code.** When you port a 1098 packet and the 7.72
   equivalent is obvious in `gameserver/src/`, cite both in the module header (`src/...` and
   `gameserver/src/...`). It costs seconds now and saves a re-investigation later.
10. **Don't expand the core→net coupling surface.** The §3.5 list is the set of core files that emit
    packets. Avoid adding new ones — route new sends through existing emission points / a thin output
    helper, so there are fewer call sites to rewire when the codec lands.

> Rule of thumb: if a change you're making would need editing in more than one place to add 7.72 later,
> push the version-specific part down into net (struct + encoder) **now**.

---

## 10. Rules for new features (share-by-default)

When building **any** new feature, decide where each part lives using these rules. Default to
**shared**; only the wire boundary is allowed to be version-specific.

**R1 — Mechanics are shared and protocol-free.** Game logic (combat, AI, conditions, skills math,
movement, loot, trade rules, spell effects, …) lives in `tfs-rust-core` and must contain **no**
opcode bytes, no `NetworkMessage` writes, and no `client_version == X` checks. It operates on game
state only.

**R2 — Only `tfs-rust-net` knows bytes.** Anything that decides byte layout, opcodes, or framing goes
in net behind the codec seam. Core never emits bytes.

**R3 — Outgoing packet pattern.** New server→client packet = (a) a neutral `XxxWire` struct holding
all fields at max width, built by core; (b) an encoder in net that writes the version's bytes. Core
builds the struct; net encodes it.

**R4 — Incoming packet pattern.** New client→server packet = a semantic `GamePacket` variant. Parsing
raw bytes happens only in `game_parse.rs`; core handles the typed variant. Core never reads raw bytes.

**R5 — Opcodes are data, centralized.** Every opcode constant goes in `protocol_opcodes.rs`
(version-keyed when it differs). Never inline a hex opcode at a call site.

**R6 — Persistence ≠ protocol.** Save/DB serialization (`item_blob.rs`, schema, `tfs-rust-db`
queries) is shared and tied to the **schema**, not the client. Keep it out of the codec. The only
version-specific DB code is auth identity (account number vs name), the character-list query, and
premium representation.

**R7 — Asset/client ids are version-bound data, not code.** Client item ids, sprite/`.dat`
signatures, and OTBM ids belong in content/data selected per server, not as constants compiled into
core or net.

**R8 — Model differences as capabilities, not scattered `if`s.** When a value or behavior differs by
version, express it as a `ProtocolCaps` field or a `ProtocolCodec` method — not as a
`if version == 772` sprinkled through logic.

**R9 — Crypto algorithm shared; framing policy is a capability.** RSA/XTEA/Adler32 *algorithms* are
shared (`rsa.rs`, `xtea*.rs`, `adler.rs`). *Whether* a checksum is applied, the XTEA slack, and the
pre-login challenge are `ProtocolCaps` flags.

**R10 — Tests are version-structured.** Every new packet gets a golden-byte test, written so a second
version can be added as a sibling module (see §7). New capabilities get a `ProtocolCaps::for_version`
assertion.

**R11 — No magic numbers in mechanics.** Combat/damage/speed/attack-speed/exp/condition constants and
formulas live in `MechanicsProfile` / `data/formulas/<version>.lua` and are Lua-tunable (§12.13).
Never bury a
balance literal in a Rust formula; the decompile/TVP value is the *default*, not a hardcode.

**R12 — Behavior from the decompile, code from ourselves.** When porting a mechanic, replicate the
decompile's *outcome* (validate against captured values) and write it in TVP/TFS style. Never copy
decompiled source. Cite both `tibia-game-master/src` (behavior) and `gameserver/src` (style) in the
module header.

### Quick "where does it go?" table

| You're building… | Crate / layer | Shared? |
|---|---|---|
| Combat formula, AI step, condition tick, skill gain | `tfs-rust-core` | ✅ shared |
| New game-state field on a creature/item | `tfs-rust-core` | ✅ shared |
| A new server→client packet's **fields** | `XxxWire` struct (net, built by core) | ✅ shared shape |
| A new packet's **byte layout / opcode** | `tfs-rust-net` codec | ❌ version-specific |
| Parsing a new client→server packet | `game_parse.rs` → `GamePacket` | ❌ version-specific (decode), ✅ semantic handling |
| Save/load format | `item_blob.rs` / `tfs-rust-db` | ✅ shared (schema) |
| Account login / char list | `tfs-rust-db` + login encoders | ❌ version-specific |
| Client item/sprite ids | content data files | data, per server |
| Crypto math | net primitives | ✅ shared |
| "Is checksum on? halved speed? addons?" | `ProtocolCaps` | ❌ version-specific flag |
| A balance constant / formula (damage, speed, exp…) | `MechanicsProfile` / `data/formulas/<version>.lua` | tunable, Lua-exposed (§12.13) |

> Enforced in `.cursor/rules/`: `TFS-protocol-versioning.mdc` (always apply),
> `TFS-wire-codec.mdc` (net + protocol common), `TFS-mechanics-profile.mdc` (core + `data/formulas/`).

---

## 11. Risks & notes

- **OTCv8 deviations are layered on 10.98**, not vanilla Tibia (e.g. the omitted item duration byte,
  the 6-vs-7 special-skill rows in `outgoing_extra.rs`, OTCv8 description string). Keep these as
  *OTCv8 sub-capabilities* of the 1098 codec, not as the generic 10.98 truth, or 7.72 work will
  inherit OTCv8 quirks by accident. See `docs/OTCLIENT_INFO.md`.
- **Content/assets are also version-bound**: `items.otb`/`items.xml` client IDs, sprite (`.spr`/`.dat`)
  signatures, and the OTBM map item ids must match the chosen client. Protocol switching alone is not
  enough to run 7.72 — flag this in `tfs-rust-content` as a follow-up (out of scope for the wire layer
  but required for an actual 7.72 server).
- **Fluid encoding differs** (7.72 `getLiquidColor` vs 10.x `fluidMap`) — verify before reuse.
- **Don't guess bytes.** Per `.cursor/rules/TFS-cpp-references.mdc`: confirm each 7.72 layout against
  `gameserver/src/` and cite file + function in the codec module headers.

---

## 12. Game mechanics — 772 is the behavioral source of truth

Versioning is not only wire format. The **game mechanics themselves differ** between 7.72 and
10.98, and the Rust port currently mirrors **TFS 1.4.2 (10.98) behavior**. For a faithful 7.72
server there is a *higher* authority than TFS: the **leaked CipSoft `tibia-game` server**
(`tibia-game-master/src/`), a decompile of the original 7.7/7.72 binary. TFS only *approximates*
CipSoft; where they disagree, **CipSoft is correct for 7.72**.

This section maps CipSoft → TVP → our Rust port so the mechanics can be mirrored "in our style"
(SlotMap entities, neutral structs, single-threaded tick, Lua-first scripting) while preserving
original behavior.

> **Three authorities, three questions:**
> - *"What bytes on the wire (772)?"* → **`gameserver/src/` only** — never decompile, never repo-root `src/`.
> - *"What should the game outcome be (772)?"* → **`tibia-game-master/src/`** (772 decompile).
> - *"What bytes / mechanics (1098)?"* → repo-root **`src/`** (TFS 1.4.2).
> - *"How do we structure NPC scripting?"* → repo-root `src/` (TFS 1.4.2 Lua npcsystem — §12.6).
>   TVP's `.ndb` engine is reference-only for conversion, not a Rust port target.
>
> **Clean-room rule:** reproduce the decompile's *outputs*, validated against captured values — do
> **not** copy its code. Write our own Rust in TVP/TFS style. (See the legal note in §12.12.)

### 12.1 The creature scheduling / "ToDo" model

CipSoft creatures are driven by a **ToDo task queue**, not a "think every N ms" loop. When the queue
drains, `IdleStimulus()` runs (the real AI tick), enqueues actions (Go/Attack/Wait/Talk), then
`Execute()` runs them, each gated by a computed delay. TFS 7.72 ported this almost verbatim
(`creature.cpp` `executeToDoEntries`); TFS 1.4.2 / our port uses scheduler-driven `onThink` + walk
deadlines instead.

| Concept | CipSoft (`tibia-game-master/src`) | TFS | Rust port |
|---|---|---|---|
| Task queue | `TCreature::ToDoList` / `Execute` (`cract.cc:728`) | `executeToDoEntries` (`creature.cpp:1386`) | walk deadlines + `check_creatures` (`walk.rs`, `creature_think.rs`) |
| Per-action delay | `CalculateDelay` (`cract.cc:846`) | `calculateToDoDelay` (`creature.cpp:1187`) | per-walk `earliest_walk_time` in `walk.rs` |
| Schedule next | `ToDoStart` → global `ToDoQueue` min-heap (`crmain.cc:12`) | per-creature `g_scheduler` event | `process_walk_due_from_wake` / `process_walk_deadlines` (`walk.rs`) |
| Idle/think | `IdleStimulus` (virtual) | `onIdleStimulus` (`monster.cpp:759`) | `creature_on_think`/`monster_on_think` (`creature_think.rs:111`) |
| Global beat | `Beat` = **200 ms** (`config.cc`) | scheduler floor **50 ms** | ~50 ms tick (`game_loop.rs`) |

**Key 7.72 difference:** CipSoft quantizes timing to a **200 ms beat**; TFS/our port use **50 ms**.
This changes effective step/attack cadence and should become a tunable (see §12.11).

### 12.2 Walking & speed

| Mechanic | CipSoft | TFS | Rust port |
|---|---|---|---|
| Speed formula | `GetSpeed = GoStrength*2 + 80` (`crmain.cc:445`) | `(2*(base+var))+80` (`creature.h:194`) | step-duration in `walk.rs` (TFS formula) |
| Step delay | `Delay=(Waypoints*1000)/Speed`, **ceil to Beat(200)** (`NotifyGo`, `cract.cc:1442`) | `50*ceil((1000*wp/speed)/50)` (`creature.cpp:185`) | `walk.rs` (50 ms model) |
| Diagonal cost | `Waypoints*3` (`cract.cc:1454`) | `waypoints*3` | matches |
| Ground cost source | tile `WAYPOINTS` attr | `ItemType.speed` of ground | ground speed in `Map`/`tile.rs` |

**Difference that matters:** the 200 ms vs 50 ms quantization means e.g. `waypoints=100, speed=200`
→ CipSoft **600 ms**/step, TFS/our port **500 ms**/step.

### 12.3 Pathfinding / auto-walk

| Mechanic | CipSoft | TFS | Rust port |
|---|---|---|---|
| Algorithm | **reverse A\*** dest→origin, `TShortway` (`cract.cc:7`) | forward A\* `getPathMatching` (`map.cpp:689`) | `get_path_matching` (`pathfinding.rs:80`) |
| Step cost | tile `WAYPOINTS`-dependent; diagonal **3×** tile cost | fixed normal **10** / diagonal **25** (+creature 30, field 180) | TFS-style fixed costs (`pathfinding.rs`) |
| Auto-walk input | client dir list → chain of `TDGo` (`receiving.cc:125`) | `playerAutoWalk` dir list | `player_auto_walk_path` (`walk.rs:969`) |

**Difference:** CipSoft path cost is **terrain-speed weighted** (prefers fast tiles); TFS/our port use
fixed 10/25 edge costs. Routes can differ over mixed terrain. A 7.72-faithful path needs the
waypoint-weighted cost model.

### 12.4 Monster AI — states, target strategy, flee, distance

CipSoft uses an explicit `STATE` machine (`SLEEPING/IDLE/UNDERATTACK/TALKING/LEAVING/ATTACKING/PANIC`,
`enums.hh`); TFS collapses it into `attackedCreature` + `isIdle` + `isAttackPanicking`
(comment in `monster.cpp:765`: "following CIP monster states").

| Mechanic | CipSoft | TFS | Rust port |
|---|---|---|---|
| Think tick | `TMonster::IdleStimulus` (`crnonpl.cc:2386`) | `Monster::onIdleStimulus` (`monster.cpp:759`) | `monster_native_on_think` (`monster_ai.rs:151`) |
| Target strategy | `Strategy[4]` roulette: nearest/lowest-HP/most-damage/random (`crnonpl.cc:2424`) | `<targetstrategy>` (`monsters.cpp:962`) | `monster_search_target` (`monster_ai.rs`) |
| Lose target | per-idle `random < LoseTarget` (`crnonpl.cc:2380`) | `changeTargetChance` (`monster.cpp:902`) | target-change logic in `monster_ai.rs` |
| Flee | `HP <= FleeThreshold`, summons never (`IsFleeing`, `crnonpl.cc:3052`) | `runonhealth`/`isFleeing` (`monster.h:143`) | `is_fleeing` (`monster_ai.rs:61`) |
| Distance keeping | hardcoded range **4** (`crnonpl.cc:2716`) | XML `targetdistance` | `monster_path_search_params` / `monster_distance_step.rs` |
| Spawn leash | `MonsterhomeInRange` (`crnonpl.cc:1497`) | `isInSpawnRange` (`monster.cpp:1652`) | `is_in_spawn_range` (`monster_ai.rs:66`) |

**Differences:** (1) CipSoft "weakest" strategy compares **current HP**; TFS compares **max HP** —
behavioral divergence on wounded targets. (2) CipSoft distance range is hardcoded 4; TFS/our port use
per-type `target_distance`. (3) Strategy[3]=RANDOM is an implicit 4th bucket in CipSoft.

### 12.5 Spawns / monsterhomes

| Mechanic | CipSoft | TFS 7.72 | Rust port |
|---|---|---|---|
| Spawn record | `TMonsterhome` (`cr.hh:771`); `monster.db` (`crnonpl.cc:1322`) | `TvpSpawn` from XML (`spawn.cpp:84`) | `spawn.rs` / `spawn_lifecycle.rs` |
| Respawn delay | `RegenerationTime` + player-count scaling (`crnonpl.cc:1311`) | `calculateSpawnDelay` (`spawn.cpp:23`) — **same scaling** | `spawn.rs` |
| Check cadence | `ProcessMonsterhomes` **1/sec rounds** (`crnonpl.cc:1395`) | `checkSpawn` ms timer (`spawn.cpp:316`) | tick-based |
| Player blocks spawn | **radius shrink** near players (`crnonpl.cc:1414`) | binary `isPlayerAround` (`spawn.cpp:368`) | check in `spawn_lifecycle.rs` |
| First-monster radius | capped to **1**, others **10** (`crnonpl.cc:1358`) | search field + retries | `spawn.rs` |

**Difference:** CipSoft *shrinks the spawn radius* when players are near (still spawns, further out);
TFS simply *blocks* the spawn if a player is in view. Our port follows TFS.

### 12.6 NPC scripting — Lua-only (no `.ndb` engine)

**Decision:** NPC logic is **TFS 1.4.2 Lua-only** for both 772 and 1098. One scripting model, no
version fork, no native behaviour-tree engine in Rust.

772 and TVP 7.72 use a condition→action tree (`TBehaviourDatabase`, `crnonpl.cc:973`) parsed
from `.npc`/`.ndb` scripts (`DEFAULT/ADDRESS/ADDRESSQUEUE/BUSY/VANISH`). TVP ports this as
`NpcBehavior` (`npcbehavior.cpp`). **We do not port that engine.** Instead we keep the existing TFS
pattern: `data/npc/scripts/*.lua` + npcsystem (`NpcEventsHandler` in `creature/npc.rs`), wired through
`tfs-rust-lua` like repo-root TFS 1.4.2.

| Concern | CipSoft / TVP 7.72 | TFS 1.4.2 / Rust port |
|---|---|---|
| Script format | `.ndb` behaviour files | Lua (`data/npc/scripts/`) |
| Trade / shop / keywords | `react` + `GiveTo`/`GetFrom` in `.ndb` | npcsystem modules + `onBuy`/`onSell` Lua callbacks |
| 772 shard content | `gameserver/data/npc/behavior/*.ndb` | **manual conversion** → Lua scripts |

**772 content path:** use TVP `.ndb` + 772 mechanics outcomes as the *behavioral reference* when writing
Lua — same clean-room rule as §12 (replicate outcomes, not transcribe `.ndb` or `npcbehavior.cpp`).
Conversion is **out-of-band content work** (edit `data/npc/scripts/`, XML spawn entries), not a Track B
Rust phase. Wire-level 7.72 has no shop window (§2.8); commerce stays script-driven in both eras.

**In scope for Rust (shared, version-agnostic):** NPC spawn/walk/say hooks, Lua event dispatch
(`NpcEventsHandler`), cylinder trade helpers the scripts call — same for 772 and 1098.

**Explicitly out of scope:** `npc_behavior.rs`, `.ndb` parser, `TBehaviourDatabase::evaluate`/`react`.

### 12.7 Combat — attack cycle, damage, armor

This is where 7.72 mechanics diverge **most** from the 10.98 model our port leans toward.

| Mechanic | 772 | TFS (classic flag) | Rust port |
|---|---|---|---|
| Attack cooldown | **fixed 2000 ms** (+200 lead-in) (`crcombat.cc:607,640`) | `getAttackSpeed()` (vocation/weapon ms) | combat skeleton (`combat/mod.rs:49`) |
| Defense cooldown | **2000 ms** gate (`crcombat.cc:236`) | `earliestDefendTime = last+2000` (`creature.cpp:500`) | not implemented |
| Melee damage | `max(0, Attack − Defense)` then armor (`crcombat.cc:647`) | `blockHit` subtracts defense then armor | not implemented |
| Damage RNG | `ProbeValue`: `((rand%100+rand%100)/2) * (5*skill+50)*weapon / 10000` (`crskill.cc:535`) | `getMaxWeaponDamage` classic — **same** (`weapons.cpp:144`) | not implemented |
| Fight-mode mods | off +20% / def −40% atk; off −40% / def +80% def (`crcombat.cc:222`) | classic `getAttackFactor` | `pvp.rs` partial |
| Armor | randomized `(Armor/2)+rand%(Armor/2)` (`crcombat.cc:285`) | classic subtracts **full** armor (`creature.cpp:532`) | not implemented |
| Distance hit | `Probe(distance*15, 90/75)`; **no defense subtract** (`crcombat.cc:731`) | `WeaponDistance::useWeapon` — aligned | not implemented |

**Key differences for 7.72:** (1) **flat 2000 ms** swing, not weapon-speed; (2) **randomized armor**
(TFS classic uses full armor); (3) balanced fight mode = **0%** modifier in CipSoft. Enable TFS
`USE_CLASSIC_COMBAT_FORMULAS` shapes as the starting point, then correct armor RNG + attack timing to
CipSoft.

### 12.8 Skills & experience

| Mechanic | CipSoft | TFS | Rust port |
|---|---|---|---|
| Level exp | `(((L-6)*L+17)*L-12)/6 * Delta` (`crskill.cc:352`) | `getExpForLevel` ×100 (`player.h:149`) | — |
| Skill-up | `Probe`/`ProbeValue`, geometric `FactorPercent` (`crskill.cc:493`) | `getReqSkillTries` `pow(mult, lvl-11)` (`vocation.cpp:140`) | — |
| Exp distribution | 20-slot `CombatList`, proportional, PvP cap **11/10** (`crcombat.cc:900`) | `distributeExperiencePoints`, `pvpExpFormula` (`creature.cpp:317`) | — |
| Attribution window | **60 rounds** (`crcombat.cc:891`) | PZ-lock ms (~60 s) | — |
| Learning points | `LearningPoints = 30` (`crcombat.cc:320`) | weapon skill point system | — |

**Difference:** CipSoft skill curves come from **race/skill `FactorPercent` + `Delta`**; TFS pulls them
from **`vocations.xml`**. Level exp polynomial is the same shape (`×100` in TFS).

### 12.9 Conditions (poison / fire / energy / haste as timer-skills)

CipSoft models conditions as **timer skills** (`TSkillPoison/Burning/Energy/GoStrength/Light/Illusion`,
`crskill.cc`) ticked by `ProcessSkills` ~1 Hz; each has `Cycle`/`Count`/`MaxCount`. TFS unifies these
into `ConditionDamage`/`Condition*` (`condition.cpp`). Our port has **merge rules only** — ticks are
not yet implemented (per `PROJECT_STATUS.md` Phase G).

| Effect | CipSoft tick | TFS | Rust port |
|---|---|---|---|
| Poison | `Damage` start, decays by `FactorPercent`, 3×3 field extend (`crskill.cc:969`) | `ConditionDamage` (`condition.cpp:1330`) | `add_condition_merge` only (`condition.rs:59`) |
| Fire | **10** dmg / 8 ticks (`crskill.cc:1057`) | `ConditionDamage` | merge only |
| Energy | **25** dmg / 10 ticks (`crskill.cc:1083`) | `ConditionDamage` | merge only |
| Haste/paralyze | `GoStrength` `MDAct` + timer (`magic.cc:226`) | `CONDITION_HASTE/PARALYZE` | merge only |

**When implementing Phase G ticks, use the CipSoft constants** (fire 10/8, energy 25/10, poison decay)
as the 7.72 truth.

### 12.10 Magic / spells

| Mechanic | CipSoft | TFS | Rust port |
|---|---|---|---|
| Spell model | `TSpellData` shape+impact (`cr.hh:55`); shape→tiles→impact (`magic.cc:400`) | `CombatArea`/`MatrixArea` (`combat.cpp`) | `matrix_area.rs` + `spell.rs` (gating only) |
| Damage scaling | `2*Level + 3*MagicLevel`, flag clamps (`magic.cc:776`) | spell min/max formulas | — |
| Runes vs instant | `CastSpell` (`magic.cc:3387`) / `RuneSpell` (`magic.cc:3641`) | `spells.cpp` instant/rune | execution missing |

**Difference:** CipSoft's universal damage multiplier is **`2·level + 3·magicLevel`** with clamp flags;
TFS 7.72 typically encodes min/max per spell. For 7.72 parity, prefer the CipSoft multiplier.

### 12.11 Which mechanics need version gating

Just as wire format is gated by `ProtocolCaps` (§4.1), version-divergent **mechanics** should be
selectable so the same core serves both eras. Model these as a parallel `MechanicsProfile`, **not**
as scattered `if version` checks. Crucially, the profile is **not hardcoded** — it is the in-memory
form of a **Lua-loaded config** (§12.13), so every value below is a tunable, not a magic number:

| Knob | 7.72 (CipSoft) | 10.98 (TFS 1.4.2) |
|---|---|---|
| Tick/beat quantization | 200 ms | 50 ms |
| Path cost model | terrain-weighted (waypoints) | fixed 10/25 |
| Attack speed | flat 2000 ms | vocation/weapon `getAttackSpeed` |
| Armor reduction | randomized `(A/2)+rand(A/2)` | full / formula |
| Fight-mode modifiers | classic ±20/40/80% | balanced 1.2× factor |
| Weakest-target metric | current HP | max HP |
| Distance keeping | hardcoded 4 | per-type `target_distance` |
| Damage formula | classic `ProbeValue` | modern level/skill formula |
| Condition tick constants | fire 10/8, energy 25/10 | TFS `ConditionDamage` defaults |
| Spawn-near-player | radius shrink | block spawn |
| Follow repath without path | yes (CipSoft `IdleStimulus`) | no (TFS `hasFollowPath`) |

> **Design tie-in:** keep mechanics shared and protocol-free (Rule R1, §10). Where behavior is
> era-specific, inject the constant/strategy via a `MechanicsProfile` value the game thread reads —
> the same capability pattern used for the wire codec. This lets one binary run a 7.72-faithful shard
> or a 10.98 shard purely by config. The profile's values come from the era's
> `data/formulas/<version>.lua` (§12.13), so a server owner retunes a shard by editing that script,
> not by recompiling.

### 12.12 Master cross-reference (CipSoft → TFS → Rust)

| Mechanic | CipSoft `tibia-game-master/src` | TFS `gameserver/src` (7.72) | Rust `crates/tfs-rust-core/src` |
|---|---|---|---|
| Step scheduling | `cract.cc` `Execute`/`CalculateDelay` | `creature.cpp` `executeToDoEntries` | `walk.rs`, `creature_think.rs` |
| Speed/step delay | `crmain.cc:445`, `cract.cc:1442` | `creature.cpp:185` | `walk.rs` |
| Pathfinding | `cract.cc:7` `TShortway` | `map.cpp:689` `getPathMatching` | `pathfinding.rs:80` |
| Monster think | `crnonpl.cc:2386` | `monster.cpp:759` | `monster_ai.rs:151` |
| Target strategy | `crnonpl.cc:2424` | `monsters.cpp:962` | `monster_ai.rs` `monster_search_target` |
| Flee / spawn range | `crnonpl.cc:3052/1497` | `monster.cpp:143/1652` | `monster_ai.rs:61/66` |
| Distance keeping | `crnonpl.cc:2716` | `monster.cpp:1073` | `monster_distance_step.rs` |
| Spawns | `crnonpl.cc:1295–1470` | `spawn.cpp:23–414` | `spawn.rs`, `spawn_lifecycle.rs` |
| NPC scripting | `crnonpl.cc:973` (`TBehaviourDatabase`) | `npcbehavior.cpp:619` (`.ndb`) | `data/npc/scripts/` (Lua), `creature/npc.rs` |
| Combat cycle | `crcombat.cc:530` | `weapons.cpp`, `creature.cpp` | `combat/mod.rs` |
| Damage/armor | `crcombat.cc:647/285`, `crmain.cc:455` | `creature.cpp:500–533` | `combat/mod.rs` (skeleton) |
| Skill/exp | `crskill.cc:352/493`, `crcombat.cc:900` | `player.cpp`, `vocation.cpp` | *(to build)* |
| Conditions | `crskill.cc:969–1090` | `condition.cpp:1330` | `condition.rs` (merge only) |
| Magic | `magic.cc:400–797` | `spells.cpp`, `combat.cpp` | `spell.rs`, `matrix_area.rs` |

> **Legal note:** `tibia-game-master` is a decompile of CipSoft's binary (released to public domain by
> its author, but CipSoft may dispute IP). Use it strictly as a **behavioral reference** to understand
> original mechanics; mirror behavior in our own Rust implementation rather than copying code. Keep
> citing the maintained `gameserver/src` (TFS 7.72) alongside it in module headers, per
> `.cursor/rules/TFS-cpp-references.mdc`.

### 12.13 No magic numbers — the Lua-tunable formula engine

**Goal:** the constants and formulas above (combat, damage, speed, attack speed, exp, skill tries,
condition ticks, spell scaling) must be **editable from the Lua API**, in keeping with TFS's
script-first ethos — not hardcoded in Rust. The decompile/TVP values become the **defaults**, and a
server owner overrides them in a script without recompiling.

**Two tiers** (keeps the hot path fast while staying fully tunable):

- **Tier 1 — tunable constants/tables (load once at startup).** Scalars and small tables read from a
  **version-specific** Lua config (`data/formulas/772.lua` or `data/formulas/1098.lua`, selected by
  `clientVersion`) into the `MechanicsProfile` struct: beat ms, attack interval, defense gate, armor
  divisor, fight-mode percents, fire `10/8`, energy `25/10`, poison decay, exp attribution window, PvP
  exp cap, distance-keep range, spell coefficients (`2*lvl+3*ml`), level-exp polynomial coefficients.
  **Zero per-call Lua cost** — they are plain Rust fields after load.
- **Tier 2 — formula override hooks (optional Lua functions).** For formulas owners most want to
  reshape, expose named callbacks. The native default reproduces the CipSoft outcome; **if** a Lua
  function is registered, it is called instead. Unregistered formulas pay **zero** runtime cost
  (an `Option<LuaFn>` check, native fast path otherwise).

**Exposed formula surface (suggested):**

| Lua hook | Inputs | Default (era) | Rust call site |
|---|---|---|---|
| `getCreatureSpeed` | base, var | `2*(base+var)+80` | `walk.rs` |
| `getStepDuration` | speed, ground, diagonal | beat-quantized delay (§12.2) | `walk.rs` |
| `getAttackSpeed` | attacker | flat 2000 ms (772) | `combat/` |
| `getWeaponDamage` | skill, attack, mode, level | `ProbeValue` (§12.7) | `combat/` |
| `getArmorReduction` | armor | randomized `(A/2)+rand(A/2)` | `combat/` |
| `getDefense` | skill, defense, mode | `ProbeValue` | `combat/` |
| `getExperienceForLevel` | level | polynomial ×Delta | skills module |
| `getReqSkillTries` | skill, level | geometric `FactorPercent` | skills module |
| `getSpellDamage` | level, magicLevel, base | `(2*lvl+3*ml)`-scaled | `spell.rs` |
| `getConditionTick` | type, round | fire 10/8, energy 25/10, poison decay | `condition.rs` |

**TFS-style shape — one file per version (`data/formulas/772.lua`, `data/formulas/1098.lua`):**

```lua
-- data/formulas/772.lua (772-faithful defaults)
formulas = {
  beatMs = 200,
  attackSpeedMs = 2000,
  armor = "randomized",         -- or "full"
  fightModes = { offensiveAtk = 1.20, defensiveAtk = 0.60,
                 offensiveDef = 0.60, defensiveDef = 1.80 },
  conditions = { fire = {dmg=10, ticks=8}, energy = {dmg=25, ticks=10} },
}

-- Optional override (Tier 2). Omit to keep the native era-faithful default.
function getWeaponDamage(skill, attack, mode, level)
  local maxv = attack * (skill * 5 + 50)
  return math.floor(((math.random(0,99) + math.random(0,99)) / 2) * maxv / 10000)
end
```

```lua
-- data/formulas/1098.lua (TFS 1.4.2 defaults — differs mainly in beat/attack/armor knobs)
formulas = {
  beatMs = 50,
  attackSpeedMs = 0,            -- 0 = use vocation/weapon getAttackSpeed()
  armor = "full",
  fightModes = { offensiveAtk = 1.20, defensiveAtk = 0.80,
                 offensiveDef = 0.80, defensiveDef = 1.20 },
  conditions = { fire = {dmg=10, ticks=8}, energy = {dmg=25, ticks=10} },
}
```

> Loader resolves `data/formulas/<clientVersion>.lua` at startup (same key as the wire codec). Missing
> file → built-in `MechanicsProfile::for_version` defaults for that era.

**Threading & performance (per `.cursor/rules/TFS-threading.mdc`):** the Lua VM lives on the **game
thread** (`tfs-rust-lua`/`mlua`), and combat/walk run on that same thread — so formula calls are
in-thread, no locks, no channels. Load Tier-1 constants once at startup; only invoke Tier-2 Lua when
an override is registered. For extreme hot paths, allow a shard to stay fully native (no overrides) —
the defaults already match the decompile.

**Rule:** new mechanic constants go into `MechanicsProfile`/`data/formulas/<version>.lua`, **never**
as a bare literal in a Rust formula. (Promoted to Rule R11, §10.)

---

## 13. Bottom line

**Almost everything is already shared.** The entire game simulation (`tfs-rust-core`), content
loaders, DB layer (minus auth identity), Lua API, and the crypto/buffer primitives are
version-agnostic and need **no** behavioral change (§3.2–§3.4). The only version-specific surface is
the **wire encode/decode boundary** plus a handful of login/transport flags.

The clean path is: (1) add a `ProtocolVersion` + `ProtocolCaps` value, (2) introduce a
`ProtocolCodec` enum/trait seam and move the *existing* 10.98 logic behind it with zero byte changes,
(2b) rewire the core emission call sites in §3.5 to go through the per-connection codec (mechanics
untouched), (3) gate transport + login by capabilities, then (4) implement `Codec772` against
`gameserver/src/`. Config (`clientVersion`) selects the codec per connection. The single-threaded
game core stays fully version-agnostic — it always fills neutral wire structs, and only the network
boundary knows the protocol.

**Two dimensions of versioning.** The *wire* is the easy half (§2–§11): a codec seam + capability
flags. The harder half is *mechanics* (§12): 7.72 behavior comes from the **CipSoft `tibia-game`
decompile**, which differs from the TFS 1.4.2 / 10.98 mechanics our port currently mirrors (200 ms
beat, terrain-weighted pathing, flat 2000 ms attack speed, randomized armor, condition-tick
constants, etc.). Treat those era-specific constants/strategies as a `MechanicsProfile` injected into
the shared core (§12.11), the same capability pattern as the wire codec — so one binary can run a
faithful 7.72 shard or a 10.98 shard by config alone.

**Three working rules that shape all of it:** (1) **Clean-room** — copy the decompile's *outcomes*,
write our own Rust in TVP/TFS style, never transcribe its code (R12). (2) **Stay TFS-style** — keep
the easy Lua-first script engine; **NPCs are Lua-only** (TFS 1.4.2 npcsystem), with TVP `.ndb` used
only as a conversion reference for 772 content (§12.6). (3) **No
magic numbers** — combat/damage/speed/attack-speed/exp/condition formulas are Lua-tunable via
`MechanicsProfile` + `data/formulas/<version>.lua` (§12.13, R11), so a shard is retuned by editing
the era's formulas script, never by recompiling.
