# NPC transcript fixture schema (NPC-0)

Black-box parity traces for imported 772 dialogue. Fixtures are **not** executed by
`GameWorld` yet; NPC-4+ differential tests will replay `steps` and assert `expected`.

## Top-level fields

| Field | Type | Required | Meaning |
|-------|------|----------|---------|
| `id` | string | yes | Stable fixture id (filename stem) |
| `description` | string | yes | What the scenario exercises |
| `npc` | string | yes | Display name |
| `sources` | string[] | yes | Behavior files under `data/npc/behavior/` (includes listed explicitly) |
| `rng_seed` | int | yes | Frozen parity RNG seed (future idle/Random differential) |
| `round_nr` | int | yes | Starting 772 `RoundNr` (advances once per second) |
| `server_ms` | int | yes | Starting harness clock (ms) |
| `game_time` | object | no | `{hour, minute}` for `%T` (12h clock in replies) |
| `world_type` | string | no | `pvp` / `non_pvp` / `pvp_enforced` (property checks) |
| `players` | array | yes | Player snapshots (see below) |
| `npc_state` | object | yes | Instance state before first step |
| `declared_rng` | array | no | Explicit `Random(lo,hi)` outcomes when seed capture is unavailable |
| `steps` | array | yes | Timed inputs |
| `expected` | array | yes | Ordered observable events |
| `cpp_refs` | string[] | yes | `crnonpl.cc` (and related) cites |

## `players[]`

| Field | Type | Meaning |
|-------|------|---------|
| `id` | string | Stable label (`P1`, `P2`) |
| `name` | string | Used for `%N` |
| `sex` | string | `male` / `female` |
| `vocation` | string | `none` / `knight` / `paladin` / `sorcerer` / `druid` |
| `pos` | `[x,y,z]` | Map position |
| `hp` | int | Hitpoints |
| `conditions` | object | e.g. `{poison: N, burning: N}` skill-timer style |
| `inventory` | `[{item_id, count}]` | Item stacks |
| `money` | int | Total gold equivalent for `CountMoney` |
| `quest_values` | object | string→int map of quest ids |

## `npc_state`

| Field | Type | Meaning |
|-------|------|---------|
| `home` | `[x,y,z]` | Start / home tile |
| `radius` | int | Roam radius |
| `state` | string | `idle` / `talking` |
| `focus` | string\|null | Player id or null |
| `topic` / `price` / `amount` / `type` / `data` | int | Per-NPC session vars |
| `queue` | `[{player, text}]` | FIFO wait queue |

## `steps[]`

Each step is an object with `at_round` (int) and `op`:

| `op` | Fields | Meaning |
|------|--------|---------|
| `say` | `player`, `text` | Normal say (`TALK_SAY`) stimulating the NPC |
| `wait_rounds` | `rounds` | Advance `RoundNr` / clock |
| `move_player` | `player`, `pos` | Relocate (range / vanish tests) |
| `remove_player` | `player` | Invalidate queued / focused player |

## `expected[]` event kinds

| `kind` | Fields | Meaning |
|--------|--------|---------|
| `situation` | `name` | `ADDRESS` / `DEFAULT` / `BUSY` / `VANISH` / `ADDRESSQUEUE` |
| `match_rule` | `summary` | Human-readable rule (documentation aid) |
| `state` | `value` | NPC state after transition (`idle` / `talking`) |
| `focus` | `player`\|null | Active interlocutor |
| `turn_to` | `player` | Facing update toward interlocutor |
| `queue` | `op`, `player`, `text?` | `push` / `pop` / `dedupe_skip` |
| `set` | `var`, `value` | Topic/Price/Amount/Type/Data |
| `say` | `text`, `delay_ms`, `byte_len` | Scheduled NPC reply |
| `mutate` | `target`, `op`, … | HP / condition / Create / Delete / money / quest |
| `todo` | `op`, `delay_ms?` | `wait` / `talk` / `start` when timing matters |

### Reply `delay_ms` (772)

From `TBehaviourDatabase::react` (`crnonpl.cc` ~1088–1113):

- `TalkDelay` starts at **1000**.
- Each `REPLY`: `ToDoWait(TalkDelay)` then `ToDoTalk`; then
  `TalkDelay += 3100 + (strlen(Response) / 2) * 100`.
- Fixture `delay_ms` is the **absolute** `TalkDelay` passed to `ToDoWait` for that reply
  (from reaction start). `byte_len` is `strlen` of the **substituted** reply bytes.

### Topic reset

Before every non-`BUSY` reaction, `Npc->Topic = 0` (`crnonpl.cc` ~1081–1083).

### Conversation timeout

`IdleStimulus`: if `LastTalk + 30 <= RoundNr` while talking → `VANISH` then idle
(`crnonpl.cc` ~1718–1727). `LastTalk` is updated on address/default speech and after
scheduled talk duration (`TalkDelay/1000 + RoundNr`).
