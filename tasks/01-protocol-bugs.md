# Australis — Known Protocol Bugs

Open wire-format issues from `GAPS.md`. All must be resolved in **Phase A** before OTClient can render the world correctly.

| # | Bug | Severity | C++ Ref |
|---|---|---|---|
| 1 | `write_item_template` missing trailing `0x00` duration byte | 🔴 Critical | `networkmessage.cpp:114` |
| 2 | Fluid/splash items never encoded (missing `fluidMap[count & 7]`) | 🔴 Critical | `networkmessage.cpp:101` |
| 3 | `sendChangeSpeed` sends 1×u32 instead of 2×u16 (baseSpeed/2 + speed/2) | 🔴 Critical | `protocolgame.cpp:2505` |
| 4 | `sendChannelMessage` field order wrong | 🔴 Critical | `protocolgame.cpp:1730` |
| 5 | `send_creature_turn` completely wrong wire format (missing 0x63 sub-header) | 🔴 Critical | `protocolgame.cpp:2404` |
| 6 | OTClient/OTCv8 detection completely missing | 🟠 High | `protocolgame.cpp:171,469` |
| 7 | `sendCancelTarget` missing `u32(0)` | 🟠 High | — |
| 8 | `send_cancel_walk` hardcodes direction 0 instead of player's actual direction | 🟠 High | — |
| 9 | Known creature eviction doesn't check visibility | 🟡 Medium | `protocolgame.cpp:744` |
| 10 | `MoveUpCreature`/`MoveDownCreature` not implemented (full map reload used) | 🟡 Medium | `protocolgame.cpp` |
| 11 | Creature visibility (ghost/invisible) not filtered in map sends | 🟡 Medium | — |
| 12 | `send_update_tile_end` non-empty path missing `GetTileDescription` | 🟡 Medium | `protocolgame.cpp:2683` |
