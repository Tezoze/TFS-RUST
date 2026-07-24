# Archived pre–NPC-2 pack layout

Moved here after the offline importer wrote declarative Lua under
`data/npc/scripts/` from **`reference/cipsoft-772/runtime/npc/`**.

| Path | Contents |
|------|----------|
| `xml/` | Former `data/npc/*.xml` with `behavior=` (337 files) |
| `behavior/` | Former `data/npc/behavior/*.npc` + `*.ndb` (data-pack split layout) |

**Do not treat this tree as 772 authority.** It includes data-pack constructs
(`String=` / `Bless` / `Town` / `Promote`) absent from `crnonpl.cc`. Re-import
from the reference corpus via:

```bash
cargo run -p tfs-rust-lua --bin import-npcs -- \
  --root reference/cipsoft-772/runtime/npc \
  --out data/npc/scripts \
  --validate-data-dir data
```

See also [`script-compat/`](script-compat/) for archived KeywordHandler `script=`
NPCs (NPC-7 migration notes).
