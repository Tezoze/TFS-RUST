# NPC system audit and implementation plan — 772 outcomes, TFS flexibility

**Status:** audit complete; NPC-0 corpus inventory + black-box fixtures frozen; NPC-1 definition model + Lua registration done; NPC-2 offline importer + reference-corpus Lua validate green; NPC-3 spawn/type integration done; NPC-4 speech/focus/matching done; NPC-5 standard actions + immediate mutation done; NPC-6 ToDo timing/movement/sleep/wake/NPC speech done
**Primary target:** exact observable 772 NPC outcomes
**Domain:** TFS-style `Npc` / `NpcType`, Lua content and userdata APIs
**Implementation:** idiomatic Rust on the game thread; LuaJIT for content hooks

## 1. Decision

Build a **first-party NPC platform**, not a port of either legacy runtime:

1. **Canonical authoring is Lua-only** under `data/npc/scripts/`.
2. Add declarative `NpcType` and `NpcDialogue` registration APIs; do not use the old `KeywordHandler` / `NpcHandler` / module library as the new foundation.
3. Add a **one-way offline importer** for full legacy `.npc` files, current split XML + behavior files, and included `.ndb` fragments. It emits deterministic declarative Lua. The server does **not** interpret `.npc` or `.ndb` at runtime.
4. Validate Lua registrations into an immutable, typed `NpcDatabase`. Standard dialogue predicates/actions execute natively in `tfs-rust-core`; optional custom predicates/actions dispatch to Lua.
5. Imported NPCs use an exact **queued single-focus** conversation policy. New/TFS NPCs may opt into per-player sessions or fully custom Lua callbacks.
6. Standard state changes are immediate and ordered. Speech, delayed state transitions, and walking use the existing `CreatureAction` / ToDo scheduler.
7. Keep the existing XML + Lua path as a temporary compatibility adapter for the nine current scripted NPCs, then migrate them to `NpcType` Lua definitions.

This gives one production runtime, exact 772 behavior for imported content, TFS-compatible Lua flexibility, static validation, hot reload, and no runtime legacy-file engine.

## 2. Why this is the best fit

| Option | Parity | Flexibility | Maintenance | Decision |
|---|---:|---:|---:|---|
| Runtime Rust `.npc`/`.ndb` interpreter | Strong if fully cloned | Low | Creates a second legacy-shaped domain | Reject |
| Generate imperative old-style NPC-handler Lua | Fragile around priority/timing | Medium | Generated control flow is hard to inspect | Reject |
| Keep the existing handler library | Does not match 772 focus, queue, priority, or timing | Medium | Global mutable tables and old callback conventions | Compatibility only |
| Declarative Lua → typed registry → native runtime + Lua hooks | Strong and testable | High | One runtime and one authoring API | **Choose** |
| Hand-write every NPC in Lua | Potentially strong | High | Hundreds of error-prone manual ports | Reject |

The file syntax is not the parity target; its **observable semantics** are. Treating legacy files as import sources prevents their parser and execution model from becoming the permanent Rust architecture.

## 3. Current-state audit

### 3.1 Existing Rust support

- `Npc` contains only `CreatureBase`, `npc_type_id`, and `wire_id`; `NpcEventsHandler` is unused. `npc_type_id` is always `0`.
- Spawn XML recognizes NPC entries, but `spawn_npc` creates hard-coded health, outfit, and speed. No NPC definition database is loaded.
- NPC idle stimulus is a no-op. Random movement, sleep/wake, focus, timeout, queueing, and facing are absent.
- Player `Say` is implemented, but NPC speech stimuli are not:
  - viewport event delivery currently resolves connected players only;
  - reference behavior sends player `TALK_SAY` to NPCs in a same-floor 3×3 search;
  - `broadcast_creature_say_viewport` currently rejects `CreatureKind::Npc`, so NPC replies cannot be emitted through it.
- Shop client packets and outgoing shop encoders exist, but the game loop has no look/buy/sell/close handlers. `Player.shop_owner` is only a placeholder.
- Inventory add/remove/count primitives, conditions, teleport, effects, spell learning, storage persistence, vocations, and monster spawning exist in pieces, but there is no unified NPC action surface.
- LuaJIT, `ScriptContext`, `LuaMutation`, scoped immediate mutation, timer events, and callback registries already provide the correct scripting boundary.
- The Lua runtime only creates an empty `Npc` class table. There is no `NpcType`, NPC loader, instance userdata, or callback registry.
- No NPC-specific tests or reload path exist.

### 3.2 Current data pack

The current `data/npc/` corpus is already a compatibility conversion rather than the original layout:

- **346** NPC XML definitions;
- **337** XML definitions use `behavior=` and have matching `behavior/*.npc` files;
- **9** XML definitions use `script=`;
- **39** shared `.ndb` fragments;
- **165** include directives across behavior files;
- legacy metadata is in XML while dialogue is in `behavior/*.npc`.

The original 772 files combine metadata and behavior in one `.npc` file:

```text
Name = "Quentin"
Sex = male
Race = 1
Outfit = (57,0-0-0-0)
Home = [32369,32239,7]
Radius = 4
GoStrength = 10
Behaviour = { ... }
```

The importer must accept both layouts. Spawn position remains a TFS spawn/map concern; imported `Home` becomes validation/migration input, not a second spawn system.

### 3.3 Existing handler-library mismatch

The current `data/npc/lib/npcsystem/` library is unsuitable as the parity layer:

- it models topics/focus primarily in Lua tables, commonly per player;
- the 772 engine has one active interlocutor and a FIFO wait queue per NPC instance;
- keyword-tree ordering is not the reference rule-selection algorithm;
- response timing and timeout behavior differ;
- shop modules favor a client shop-window flow, while legacy behavior files trade through dialogue actions;
- global mutable tables and implicit callbacks are difficult to validate or hot-reload safely.

Keep a narrow adapter only until the nine scripted NPCs are migrated.

## 4. Confirmed 772 outcomes to preserve

C++ outcome source: `tibia-game-master/src/crnonpl.cc`, `crmain.cc`, `cr.hh`, `strings.cc`, `operate.cc`, `script.cc`, and `main.cc`. TFS domain/API source: `npc.cpp`, `npc.h`, `luascript.cpp`, and the current data layout.

### 4.1 Loading and syntax

- Startup scans `.npc` files; each full file defines metadata plus `Behaviour = { ... }`.
- `@"file.ndb"` is a lexical include resolved relative to the including file.
- Includes are flattened at their source location and preserve declaration order.
- Identifiers are case-insensitive; quoted response text preserves bytes.
- `#` starts a line comment.
- Expressions support literals, `%1`/`%2`, `+`, `-`, `*`, comparisons, variables, player reads, and functions such as item count, money count, random, spell, and quest-value reads.
- Parser diagnostics must retain source file, include stack, line, and column. Runtime loading must never silently skip a rule.

### 4.2 Speech stimulus

- Only normal player say stimulates NPC dialogue.
- Candidate NPCs are same-floor creatures in a 3×3 range.
- Candidate order follows the reference spatial scan: X blocks within each Y block, then the creature chain's order inside a block. Do not replace this with a `CreatureId` sort until a trace proves equivalence; global RNG consumption can make order observable.
- NPCs ignore their own speech.
- Whisper, yell, private messages, and channels do not enter the classic dialogue matcher.

### 4.3 Conversation ownership

Imported NPCs use one active interlocutor:

- idle NPC + speech → `ADDRESS`;
- focused player + speech → normal/default situation;
- another player + speech → `BUSY`, temporarily evaluated as that player, then focus restored;
- `Queue` during `BUSY` appends the player and original message once;
- queue is FIFO and deduplicated by player;
- queued players are removed when invalid or out of range;
- when idle, the first valid queued address is replayed as queued-address input;
- leaving range, removal, or timeout runs `VANISH` and releases focus;
- timeout is **30 rounds**, and `RoundNr` advances once per second;
- `Topic`, `Price`, `Amount`, item type, and generic data are per NPC instance, not per player;
- topic resets before every non-busy reaction.

A separate `per_player` policy may be offered for new content, but imported definitions must never use it implicitly.

### 4.4 Rule matching

For each rule in declaration order:

1. Match conditions left-to-right.
2. Text conditions search sequentially through the remaining message.
3. Text matching is case-insensitive and begins at an alphanumeric word boundary.
4. A trailing `$` requires a boundary after the matched term.
5. `%1` and `%2` capture numeric words; captured values are capped at `500`.
6. Select the matching rule with the greatest condition count; equal counts keep the earlier rule.
7. `!` selects the current rule immediately once preceding conditions match.
8. `*` executes the previous declared rule's actions; it is not a new keyword lookup.

Do not use the later priority-sort reconstruction; it changes rule selection.

### 4.5 Action ordering and timing

- Actions execute left-to-right with immediate visibility and **no transaction/rollback**.
- A failed action logs its error; already-applied actions remain applied.
- Standard replies substitute `%N` (player name), `%A` (amount), `%P` (price), and `%T` (game time).
- First reply is scheduled after `1000 ms`.
- Each next reply delay adds `3100 + (response_byte_len / 2) * 100 ms`.
- Final wait is scheduled after the last action; `LastTalk` is moved into the future by the scheduled talk duration.
- State changes before any queued work may apply immediately; after queued speech begins, state changes are queued through ToDo.
- Item/money/quest/vocation/condition/effect/teleport/spell/summon mutations occur in action order.
- Legacy dialogue trading remains dialogue-based. Do not silently replace it with a shop window.

### 4.6 Movement and facing

- Addressing a player turns the NPC toward that player.
- Movement of either side updates facing while focused.
- Idle movement tries up to ten random cardinal destinations using the parity RNG.
- A valid step stays on the home floor, inside the configured radius, outside houses, and off blocked/avoid tiles.
- Successful or unsuccessful roaming schedules the next idle opportunity after `2000 ms`.
- NPCs sleep when no relevant players are nearby and wake from player movement stimulus.

### 4.7 Era/profile placement

Put tunable outcomes in `MechanicsProfile`, not bare literals in core:

- conversation timeout rounds;
- initial reply delay;
- inter-reply base delay and byte-length factor;
- numeric capture cap;
- idle movement attempts and delay;
- classic speech stimulus range.

The conversation policy and authored rules belong to the NPC definition, not `clientVersion` checks.

## 5. Target architecture

```text
legacy .npc/.ndb + current XML
          │ one-way offline import
          ▼
data/npc/scripts/*.lua
  NpcType + NpcDialogue registrations
          │ LuaJIT startup loader + validation
          ▼
Arc<NpcDatabase> (immutable definitions)
          │ definition id
          ▼
SlotMap<CreatureId, CreatureKind::Npc>
  NpcRuntimeState (game thread only)
          │ standard actions        │ custom hooks
          ▼                         ▼
GameWorld native APIs       EventDispatcher → Lua callback
          │
          ▼
codec-neutral outgoing events → tfs-rust-net codecs
```

### 5.1 Definition layer (`tfs-rust-content`)

Add neutral, immutable data types:

- `NpcDatabase`: normalized spawn name → `NpcTypeId` → `Arc<NpcDefinition>`;
- `NpcDefinition`: name, appearance, health, movement, flags, speech bubble, parameters, voices, optional dialogue program, optional callback keys, optional shop definition;
- `DialogueProgram`: policy, ordered rules, source spans;
- `DialogueRule`: predicates + ordered actions;
- `DialoguePredicate`: situation, words, numeric capture, expression, property, custom callback;
- `DialogueAction`: say, set session value, change focus state, player/item/money/storage/condition/effect/teleport/spell/summon operations, custom callback;
- `DialogueExpr`: typed integer expression tree with checked import-time validation;
- `SourceSpan`: generated Lua file plus original source file/line for diagnostics.

These are TFS-domain definitions, not public legacy parser types. Importer AST types remain private to the offline tool.

### 5.2 Authoring layer (`tfs-rust-lua`)

Provide a compact first-party API:

```lua
local npc = NpcType("Quentin")

npc:appearance({ lookType = 57 })
npc:movement({ radius = 4, speed = 10 })
npc:dialogue(NpcDialogue({
    policy = "queued_single_focus",
    rules = {
        {
            when = {
                { situation = "address" },
                { words = { "hello$" } },
                { select = true }
            },
            actions = {
                { say = "Welcome, adventurer %N!" }
            }
        }
    }
}))

npc:onCustomAction("quest_reward", function(context)
    -- TFS userdata APIs; immediate mutations through LuaMutation.
end)

npc:register()
```

Requirements:

- constructors collect pending definitions; `register()` validates and freezes them;
- predicate and action tables are ordered arrays; never depend on Lua hash-table iteration;
- no global current-NPC state;
- no implicit `getNpcCid()` dependency;
- callbacks receive typed `NpcRef`, `PlayerRef`, event data, and session access;
- standard predicates/actions are declarative and native;
- definitions store opaque callback ids while `LuaEventDispatcher` owns the `mlua::RegistryKey`s;
- custom predicates are read-only; custom actions use scoped immediate mutation;
- custom hooks are named and cached as registry callbacks;
- TFS userdata methods remain available for community scripts;
- all runtime Lua calls stay on the game thread.

### 5.3 Runtime state (`tfs-rust-core`)

Replace `npc_type_id: u32` with a typed definition id and add per-instance state:

```text
Npc
├── base: CreatureBase
├── definition: NpcTypeId
├── wire_id: u32
└── runtime: NpcRuntimeState
    ├── activity: Sleeping | Idle | Talking | Leaving
    ├── conversation: QueuedSingleFocus | PerPlayer
    ├── topic / price / amount / item_type / data
    ├── last_talk_round
    ├── home_position / radius / next_walk
    └── active_shop_sessions
```

Use `CreatureId` in focus/queue/session references and `VecDeque` for FIFO behavior. Generational keys safely invalidate removed players; no `Rc<RefCell<_>>`, raw references, locks, or Tokio tasks.

`QueuedSingleFocus` preserves reference outcomes. `PerPlayer` stores explicit `HashMap<CreatureId, DialogueSession>` state and is opt-in content behavior.

### 5.4 Runtime execution split

Keep matching/evaluation separate from mutation:

1. Snapshot only the small read set needed to choose a rule.
2. Produce the selected rule id and numeric captures without borrowing an entity mutably.
3. Execute actions in declaration order against `&mut GameWorld`.
4. Queue speech/state/walk actions through existing ToDo helpers.
5. Invoke custom Lua predicates/actions only through `fire_npc_*` scope helpers.

Do not precompute a transaction: partial sequential effects are observable reference behavior.

### 5.5 Event and Lua boundary

Retire the unused `NpcEventsHandler: Send + Sync` surface. It conflicts with the game-thread-only, `!Send` Lua runtime.

Extend the existing `EventDispatcher` and scoped Lua bridge with:

- `on_npc_appear` / `on_npc_disappear`;
- `on_npc_move` / `on_player_move_near_npc`;
- `on_npc_say` for custom definitions;
- `on_npc_think` only when registered;
- `on_npc_custom_predicate` and `on_npc_custom_action`;
- `on_npc_shop_buy` / `sell` / `close`.

Each callback gets a matching `fire_npc_*` helper in `lua_scope.rs`; scripts may observe mutations before the callback returns.

### 5.6 Chat integration

Add a dedicated speech-stimulus pass after normal SAY processing:

- query the spatial grid for same-floor NPCs in profile range;
- preserve reference block/chain scan order while deduplicating candidates;
- call `npc_talk_stimulus` for each candidate;
- do not route NPC listeners through connection-only spectator lists;
- preserve player chat packet/event ordering before NPC reactions;
- permit `CreatureKind::Npc` in NPC speech broadcast encoding.

### 5.7 Optional shop windows

Shop windows are a TFS flexibility feature, not the imported default:

- change `Player.shop_owner` to `Option<CreatureId>`;
- store a validated shop snapshot/session id so reload cannot misroute packets;
- implement look/buy/sell/close game-loop arms;
- resolve client item ids through `ItemDatabase` and the active codec;
- perform capacity, money, subtype, backpack, equipped-item, and count checks in core;
- apply item/money changes immediately, then refresh sale counts;
- close sessions on NPC/player removal, range loss, reload, or script error;
- imported dialogue shops continue to use dialogue actions unless explicitly migrated.

### 5.8 Hot reload

Reload is an explicit TFS extension:

1. Build and validate a new registry off to the side.
2. If any definition fails, keep the old registry unchanged.
3. Close shop windows.
4. Swap the `Arc<NpcDatabase>` on the game thread.
5. Rebind live NPCs by normalized definition name.
6. Reset transient conversations and queues; preserve `CreatureId`, position, and spawn ownership.
7. Broadcast appearance/speed changes only when fields changed.

No I/O task may mutate `GameWorld`. File reads may occur off-thread and return owned source bytes through `GameCommand`; Lua execution, callback registration, live registry validation, and the final swap stay on the game thread.

## 6. Importer and migration design

### 6.1 Inputs

Support two import modes:

- full legacy `.npc` files with metadata and inline/included behavior;
- current `data/npc/*.xml` + `data/npc/behavior/*` split files.

The importer must:

- resolve includes relative to the including file;
- reject traversal outside the selected input root;
- detect cycles and excessive include depth;
- preserve declaration order and previous-rule links across include boundaries;
- normalize ISO-8859-1 XML and legacy text deterministically to UTF-8;
- preserve exact dialogue text bytes after decoding;
- map server item ids without guessing;
- report unsupported identifiers/actions as errors, not warnings;
- attach original source spans to every generated rule/action;
- generate stable Lua formatting so reruns have clean diffs.

### 6.2 Output

Generate committed Lua definitions under `data/npc/scripts/`. Do not load generated cache files from `target/` or a writable runtime directory.

The generated definition should be readable declarative data, not a chain of imperative keyword-handler calls. Shared `.ndb` fragments may become reusable Lua modules only when reuse does not alter declaration order; otherwise flatten them with source-span metadata.

### 6.3 Compatibility policy

- Runtime `.npc` / `.ndb` loading: **not supported**.
- Existing XML `script=` NPCs: supported during migration.
- Existing XML `behavior=` NPCs: importer input only; remove runtime dependence once generated definitions pass parity tests.
- Existing handler library: isolated compatibility layer, then removable after all nine scripts migrate.
- New NPCs: Lua `NpcType` / `NpcDialogue` only.

## 7. Phased implementation

### NPC-0 — Freeze parity evidence and corpus

- [x] Add a feature inventory generated from all 337 behavior files, 39 shared fragments, and 165 include edges.
- [x] Record every identifier, expression function, action, substitution, include edge, encoding, and unsupported construct.
- [x] Build transcript fixtures from representative files:
  - simple greeting/farewell;
  - Quentin healing/conditions;
  - Zebron numeric capture/random/money;
  - bank include and change-making;
  - explorer quest include;
  - guard include;
  - multi-reply timing;
  - two-player busy/queue/vanish.
- [x] Add a small reference trace harness around `TBehaviourDatabase::react`, `TalkStimulus`, and `IdleStimulus`, or capture equivalent black-box traces.
- [x] Freeze RNG seed, `RoundNr`, player state, inventory, and expected ordered events.

**Gate:** no runtime work begins until corpus coverage is 100% and unresolved parser constructs are listed.

**NPC-0 deliverables:** `scripts/npc_corpus_inventory.py` → `tasks/npc-corpus-inventory.{json,md}`; black-box fixtures under `tests/fixtures/npc/` (+ `scripts/validate_npc_fixtures.py`). Live C++ harness deferred.

### NPC-1 — Definition model and Lua registration

Affected files:

- new `crates/tfs-rust-content/src/npcs.rs` or `npcs/` module;
- `crates/tfs-rust-content/src/lib.rs`;
- new `crates/tfs-rust-lua/src/npc_type.rs`, `npc_dialogue.rs`, `npc_loader.rs`;
- `crates/tfs-rust-lua/src/runtime.rs` and `lib.rs`.

Work:

- [x] Add typed ids, definitions, rules, expressions, source spans, policies, voices, callbacks, and shops.
- [x] Register `NpcType` / `NpcDialogue` constructors and methods.
- [x] Load `data/npc/scripts/**/*.lua` deterministically.
- [x] Validate duplicate names, invalid item ids, impossible expressions, missing callbacks, and malformed definitions.
- [x] Freeze definitions into `Arc<NpcDatabase>` and expose no mutable definition references.
- [x] Add loader unit tests and duplicate/error diagnostics tests.

**Gate:** a handwritten declarative NPC loads without `GameWorld` and produces a stable definition snapshot.

**NPC-1 deliverables:** `crates/tfs-rust-content/src/npcs/`; `crates/tfs-rust-lua/src/{npc_type,npc_dialogue,npc_loader}.rs`; smoke `data/npc/scripts/greeting.lua`.

### NPC-2 — Offline legacy importer

Affected files:

- `crates/tfs-rust-content/src/npc_import/` (lexer/parser/includes/lower/emit);
- `tfs-rust-lua` binary `import-npcs`;
- generated `data/npc/scripts/*.lua` only after validation (not committed wholesale yet).

**Authority corpus:** `reference/cipsoft-772/runtime/npc/` (337 full `.npc` + 39 `.ndb`). Gate does **not** use the old data-pack split when it diverges (`String=`/`Bless`/`Town`/`Promote` are pollution — hard-rejected). That pack now lives under `data/npc/archive/`.

Work:

- [x] Implement lexer with source spans, comments, escapes, include stack, coordinates, outfits, identifiers, numbers, and operators.
- [x] Parse metadata, ordered rules, conditions, expressions, actions, `!`, and `*`.
- [x] Import current XML metadata/parameters and behavior references (secondary `--split-xml` mode).
- [x] Emit deterministic declarative Lua.
- [x] Run generated Lua through the real `LuaRuntime` validator (`import-npcs --validate-data-dir`).
- [x] Add parse-all tests for the complete corpus and golden generation tests.
- [x] Fail the command if any NPC or included fragment is unsupported.

**Gate:** all current **reference** behavior definitions import and register with zero dropped rules/actions.

### NPC-3 — Spawn/type integration

Affected files:

- `crates/tfs-rust-core/src/creature/npc.rs`;
- `crates/tfs-rust-core/src/game_world.rs`;
- `crates/tfs-rust-core/src/spawn_lifecycle.rs`;
- `crates/tfs-rust-core/src/run_server.rs`;
- test constructors that instantiate `Npc`/`GameWorld`.

Work:

- [x] Add `Arc<NpcDatabase>` to `GameWorld`.
- [x] Resolve spawn names case-insensitively to typed definition ids.
- [x] Instantiate health, outfit, speed, flags, movement home/radius, speech bubble, and runtime state from definitions.
- [x] Reject unknown NPC spawns with a precise source/location error.
- [x] Preserve SlotMap ids and existing spawn ownership rules.
- [x] Add spawn appearance, lookup, default, and unknown-name tests.

**Gate:** spawned NPCs display correct name/outfit/health/speed and contain no placeholder type id.

### NPC-4 — Speech, focus, rule matching, and queue

Affected files:

- new `crates/tfs-rust-core/src/npc/` runtime modules;
- `game_world_chat.rs`;
- `game_world_spectators.rs` or a reusable creature-grid collector;
- `idle_stimulus.rs`;
- movement/removal notification paths.

Work:

- [x] Add same-floor normal-say NPC stimulus collection at profile range.
- [x] Implement queued-single-focus and opt-in per-player policies.
- [x] Implement address/default/busy/vanish/queued-address transitions.
- [x] Implement deterministic matching, boundaries, numeric capture, condition-count selection, `!`, and `*`.
- [x] Remove invalid/out-of-range queued players and deduplicate queue entries.
- [x] Turn NPCs toward active interlocutors.
- [x] Add exact two-player queue, range, timeout, topic-reset, tie-break, and capture tests.

**Gate:** pure dialogue/state traces match NPC-0 fixtures before adding mutating actions.

**NPC-4 deliverables:** `crates/tfs-rust-core/src/npc/` (`words`/`match_rule`/`expr`/`react`/`focus`/`stimulus`); `MechanicsProfile::npc` + `data/formulas/{772,1098}.lua` knobs; SAY → `deliver_npc_say_stimuli`; move/timeout hooks. Mutating actions emit `DeferredAction` until NPC-5; reply ToDo drain lands in NPC-6.

### NPC-5 — Standard actions and immediate mutation

Affected files:

- new NPC action applier in core;
- `player/inventory/util.rs` and existing cylinder helpers;
- player storage/quest APIs;
- condition, effect, teleport, vocation, spell, and summon integration;
- save dirty-state plumbing where needed.

Work:

- [x] Implement session variables and response substitutions. *(done in NPC-4)*
- [x] Reuse existing item count/add/remove cylinder APIs; do not create NPC-only inventory logic.
- [x] Add TFS-shaped money helpers with exact 772 denomination/change outcomes for imported actions.
- [x] Expose storage/quest reads and immediate writes on the live player; mark persistence dirty.
- [x] Wire HP, poison/fire condition removal/application, effects, vocation/promotion, spell learning, summon, teleport, and home-town/start-position actions.
- [x] Execute actions left-to-right without rollback.
- [x] Log failures with NPC, player, rule source span, and action index.
- [x] Add partial-failure ordering tests.

**Gate:** mutating transcript fixtures match ordered world changes and packets.

**NPC-5 deliverables:** `npc/actions.rs` + `npc/host.rs`; `player/inventory/money.rs` (2148/2152/2160 + `CalculateChange`); live `EvalContext` reads in `focus.rs`; `DialogueEvent::Mutate` replacing deferred stubs (Custom stays deferred until NPC-7).

### NPC-6 — ToDo timing, movement, sleep/wake, and NPC speech

Affected files:

- `creature_todo.rs` / `idle_stimulus.rs`;
- walk and creature movement notification modules;
- `game_world_spectators.rs`;
- `data/formulas/772.lua`, `1098.lua`, and mechanics structs.

Work:

- [x] Add profile fields for NPC timing/range constants; derive 772 values from `tibia-game-master` and verify 1098 defaults against repo-root TFS before writing `1098.lua`.
- [x] Permit NPC speakers in creature-say encoding and fan-out.
- [x] Schedule reply waits, talk actions, and state changes with exact byte-length timing.
- [x] Implement 30-round timeout using logical `round_nr`.
- [x] Implement ten-attempt cardinal roaming with parity RNG and existing tile queries.
- [x] Enforce home floor/radius/house/avoid/block constraints.
- [x] Implement player-driven sleep/wake and move/disappear vanish stimuli.
- [x] Add deterministic timing/RNG/ToDo queue tests.

**Gate:** NPC-0 timing and movement traces match exactly under a fixed RNG seed.

**NPC-6 deliverables:** `CreatureAction::Talk(String)` + `ChangeNpcState`; NPCs on ToDo/`IdleStimulus`; reply Wait→Talk→trailing Wait scheduling; NPC say fan-out; roam/sleep/wake + `NpcTuning` keepalive/roam/sleep knobs; unit tests for ToDo timing, keepalive, roam RNG, sleep/wake, Tom multi-reply delays.

### NPC-7 — Lua custom callbacks and TFS compatibility

Affected files:

- `event_dispatcher.rs`;
- `lua_event_dispatcher.rs`;
- `lua_scope.rs`;
- `tfs-rust-lua` NPC userdata and callback modules;
- `data/npc/scripts/` migrated custom NPCs.

Work:

- [x] Replace unused `NpcEventsHandler` with `EventDispatcher` NPC methods.
- [x] Add `NpcRef` userdata storing typed ids only.
- [x] Add scoped `fire_npc_*` helpers and immediate mutation coverage.
- [x] Implement custom predicate/action and appear/disappear/move/say/think callbacks.
- [x] Port the minimum TFS NPC userdata/API surface used by current and community scripts.
- [x] Run custom `onThink` only for definitions that register it.
- [x] Migrate the nine `script=` NPCs away from the old handler library.
- [x] Add script-error isolation and same-callback read-after-write tests.

**Gate:** all canonical NPC content runs without loading `data/npc/lib/npcsystem/`.

### NPC-8 — Optional shop-window subsystem

Affected files:

- new `game_world_npc_shop.rs`;
- `game_loop.rs` packet arms;
- `creature/player.rs` session type;
- existing net shop encoders and codec tests;
- Lua NPC shop callbacks.

Work:

- [ ] Implement open/list/sale-count/inspect/buy/sell/close lifecycle.
- [ ] Use `CreatureId` ownership and validated immutable shop definitions.
- [ ] Reuse core item/money/capacity/cylinder operations.
- [ ] Add era-specific wire snapshot tests: 772 bytes only from `gameserver/src/`, 1098 bytes from repo-root `src/`.
- [ ] Close sessions on range loss, removal, logout, reload, and replacement.
- [ ] Keep imported dialogue trades unchanged by default.

**Gate:** shop-window NPCs work on both codecs without changing imported NPC transcripts.

### NPC-9 — Atomic reload, diagnostics, and rollout

Affected files:

- NPC loader/registry modules;
- game control command and admin reload path;
- tracing/state inspection tooling;
- generated definitions and compatibility cleanup.

Work:

- [ ] Implement parse/validate/swap reload with rollback on failure.
- [ ] Close shops and reset transient sessions at the swap boundary.
- [ ] Add structured trace events for stimulus, candidate rules, selection, actions, queue, focus, and timing.
- [ ] Add an offline `validate-npcs` command for CI.
- [ ] Differential-test every generated definition; classify any approved content deviations.
- [ ] Remove XML `behavior=` runtime support and old handler-library dependency only after all gates pass.
- [ ] Profile large NPC/player crowds; optimize grid collection/callback dispatch only from measurements.

**Gate:** full corpus validates, all parity fixtures pass, reload is atomic, and compatibility removal causes no content loss.

## 8. Test strategy

### 8.1 Parser/import tests

- full-file metadata and split XML import;
- nested/invalid/cyclic/path-traversal includes;
- source spans through includes;
- ISO-8859-1 and UTF-8 text;
- comments, escapes, negative numbers, coordinates, precedence;
- `!` and `*` across include boundaries;
- deterministic output and parse-all corpus test.

### 8.2 Pure matcher tests

- case-insensitive sequential word matching;
- `$` boundary behavior and punctuation;
- `%1`/`%2`, 500 cap, and reference pointer advancement;
- maximum-condition selection and declaration-order ties;
- topic reset and busy temporary interlocutor;
- substitutions and response-length limits.

### 8.3 Runtime tests

- same-floor 3×3 SAY stimulus only;
- focus, busy, queue dedupe/FIFO, invalid queue removal, vanish;
- 30-second logical timeout under lagged wall clock;
- immediate ordered actions and partial failures;
- item/money/storage/condition/vocation/spell/summon/teleport outcomes;
- deterministic random and idle movement;
- NPC speech packet/event order;
- removal/logout/reload cleanup.

### 8.4 Lua tests

- definition registration/validation;
- callback lookup caching;
- custom predicate/action return contracts;
- read-after-immediate-mutation in one callback;
- callback errors do not crash or corrupt focus/session state;
- no Lua callback on standard-only hot paths.

### 8.5 Network/shop tests

- byte snapshots per codec;
- item id/subtype conversion;
- capacity, money, count, backpack, and equipped-item flags;
- stale owner/session rejection;
- sale-list refresh after inventory changes;
- close behavior on every lifecycle edge.

### 8.6 Verification commands

Run after each phase:

```bash
rtk cargo check --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo test --workspace
```

Importer/rollout gates should additionally run the planned validator and complete transcript differential suite.

## 9. Risks and controls

| Risk | Control |
|---|---|
| Later reconstruction differs from 772 outcomes | Cite and test `tibia-game-master` outcomes; use later code only for TFS domain/wire surfaces |
| Generated Lua reproduces syntax but not semantics | Differential transcript tests target behavior, timing, ordered mutations, and packets |
| A new declarative API becomes another opaque DSL | Keep it typed, small, source-spanned, inspectable, and Lua-extensible |
| Standard and Lua actions diverge | One core mutation API; Lua uses the same immediate applier paths |
| Multi-player flexibility changes imported NPC behavior | Explicit policy; imported content always uses queued single focus |
| Shop UI changes classic trading | Shop is opt-in; importer preserves dialogue actions |
| Hot reload corrupts live state | Build/validate first, atomic swap, reset transient sessions, preserve ids/positions |
| Lua stalls the game thread | Native standard path, callback caching, callback-only dispatch, profiling |
| Entity lifetime bugs | Typed SlotMap ids in all focus/queue/shop/callback state |
| Legacy paths escape the data root | Canonicalize importer paths and reject traversal/cycles |
| Bare era constants drift | Put tunable timings/ranges in `MechanicsProfile` and formulas Lua |

## 10. Completion criteria

The NPC system is complete when:

- all 346 current NPC definitions load through canonical Lua registrations;
- all 337 behavior NPCs import with zero dropped rules/actions;
- the nine custom scripts no longer require the old handler library;
- imported dialogue traces match 772 for selection, focus, queue, timeout, timing, RNG, actions, movement, and packets;
- custom Lua NPCs can use TFS userdata APIs and immediate mutations;
- shop windows work as an opt-in TFS feature on both codecs;
- NPC reload is atomic and safe;
- no runtime `.npc`/`.ndb` interpreter, version-suffixed core API, direct `GameWorld` access from I/O tasks, or entity reference/lock graph is introduced;
- every substantial new Rust module cites its TFS domain source and 772 outcome functions in the required module header;
- `cargo check`, clippy, workspace tests, corpus validation, and differential fixtures pass.
