# Archived KeywordHandler `script=` NPCs (pre–NPC-7)

Recovered from git `0a522c3^` after NPC-2 removed live `data/npc/*.xml`.

These used `data/npc/lib/npcsystem/` (`KeywordHandler` / `NpcHandler`). That library is
**not loaded** at runtime after NPC-7.

| Legacy NPC | Script | Disposition |
|------------|--------|-------------|
| Captain | `ship.lua` | Migrated → `data/npc/scripts/captain.lua` |
| Banker | `bank.lua` | Migrated (core deposit/withdraw) → `data/npc/scripts/banker.lua` |
| Alice | `bless.lua` | Not migrated — `Bless` hard-rejected by importer |
| The Forgotten King | `promotion.lua` | Not migrated — `Promote` hard-rejected |
| Deruno / Riona / Tyoric / Eryn | `default.lua` / `runes.lua` | Deferred to **NPC-8** (shop window) |
| The Oracle | `The Oracle.lua` | Duplicate of imported 772 `the_oracle.lua` |

Do not re-enable `dofile('data/npc/lib/npcsystem/…')` in `data/npc/lib/npc.lua`.
