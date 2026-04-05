Protocol Audit: Rust (tfs-rust-net) vs TFS 1.4.2 C++
Scope: Full byte-level comparison of server→client and client→server game protocol
Protocol: Tibia 10.98 (OTClient v8)
Date: 2026-04-05

Executive Summary
Login and basic movement work because the handshake, XTEA, and map description are largely correct. However, the world is not rendering items/buildings correctly because of a critical item encoding mismatch — the Rust side omits the trailing duration byte (0x00) that the C++ addItem always appends. Additionally, OTClient/OTCv8 detection is completely absent from the Rust side, sendChangeSpeed sends the wrong field count, sendChannelMessage has a wrong field order, and fluid items are never handled. Below is every gap found, ranked by severity.

🔴 CRITICAL — Causes Rendering Corruption / Client Crash
1. addItem Duration Byte — Missing Trailing 0x00
C++ (networkmessage.cpp)	Rust (item_encode.rs)
Template item	clientId + 0xFF + stackable/fluid + animation + 0x00 (duration)	clientId + 0xFF + stackable + animation — no 0x00
Live item	Full duration logic (0x00/0x01+u32+u8)	N/A (no live items yet)
Why it matters: In C++, addItem(uint16_t id, uint8_t count, bool withDescription) on 
networkmessage.cpp:114
 always writes addByte(0x00) at the end for template items. The Rust 
write_item_template
 deliberately omits it, with a comment saying OTClient v8 stock doesn't read it.

However, your TFS C++ source has otclientV8 = 1 as the default in protocolgame.h:327, meaning addItem is called with withDescription = otclientV8 → withDescription = true for virtually all items. This changes the wire format to include a description string + duration. The Rust side doesn't account for this at all.

CAUTION

This is the most likely cause of your "world not rendering items/buildings correctly" bug. Every item on every tile is 1+ bytes short, causing all subsequent items, creatures, and skip-counts on that tile (and cascading to the entire map stream) to be misaligned.

Fix: Either:

(a) Always append 0x00 for duration in write_item_template (matching default withDescription=false path), OR
(b) Implement the full withDescription=true path matching otclientV8 behavior (description string + duration)
2. Fluid/Splash Items — Never Encoded
C++ addItem 
networkmessage.cpp:101-103
:

cpp
if (it.stackable) {
    addByte(count);
} else if (it.isSplash() || it.isFluidContainer()) {
    addByte(fluidMap[count & 7]);  // ← MISSING in Rust
}
Rust write_item_template only handles stackable. If an item is a fluid container or splash (potions, troughs, blood puddles), the client expects a fluid sub-type byte that is never sent.

WARNING

Fluid containers on the map will be silently misaligned, corrupting the byte stream for everything after them on that tile.

3. sendChangeSpeed — Missing baseSpeed Field
C++ 
protocolgame.cpp:2505-2512
Rust 
outgoing_extra.rs:298-304
Fields	0x8F + u32(id) + u16(baseSpeed/2) + u16(speed/2)	0x8F + u32(id) + u32(speed)
Rust sends one u32 where C++ sends two u16s. This is a 2-byte misalignment and the speed value itself is wrong (not halved). The client will misread the packet.

4. sendChannelMessage — Wrong Field Order
C++ 
protocolgame.cpp:1730-1741
:

0xAA | u32(0) | string(author) | u16(0) | u8(type) | u16(channel) | string(text)
Rust 
outgoing_extra.rs:452-461
:

0xAA | u32(0) | u8(speak) | u16(channel) | string(author) | string(text)
CAUTION

The Rust version puts speak_type before channel, and swaps author position. The C++ version writes author then u16(0) (level) then type then channel then text. Chat messages will be completely garbled.

5. send_creature_turn — Wrong Wire Format
C++ 
protocolgame.cpp:2404-2424
:

0x6B | position_or_ffff_id | u16(0x63) | u32(id) | u8(direction) | u8(walkthrough)
Rust 
outgoing_extra.rs:470-476
:

0x6B | u32(creature_id) | u32(stack_pos)
The Rust version is completely wrong — it sends two u32s where C++ sends a complex creature-turn-on-tile message with the 0x63 (known creature) sub-header, direction, and walkthrough byte. Creature turns will not work at all.

🟠 HIGH — Causes Functional Gaps or Wrong Behavior
6. OTClient / OTCv8 Detection — Completely Missing
The C++ source has a multi-layered client detection system:

Detection	C++ Location	Rust
Operating System enum (operatingSystem >= CLIENTOS_OTCLIENT_LINUX)	
protocolgame.cpp:171
❌ Not parsed from first packet
OTCv8 string probe ("OTCv8" after auth)	
protocolgame.cpp:469-472
❌ Not parsed
OTCv8 version number (253, 260, 261…)	
protocolgame.cpp:471
❌ Not stored
Login OTCv8 probe	
protocollogin.cpp:244-249
❌ Not parsed
Impact:

addItem's withDescription flag is driven by otclientV8 in C++ — without detecting this, the Rust server can't match the item encoding the client expects
The sendFeatures/sendExtendedOpcode preamble depends on operatingSystem — the Rust server always sends OTCv8 features regardless
registerCreatureEvent("ExtendedOpcode") should only trigger for OTClient
7. sendCancelTarget — Missing u32(0)
C++ writes 0xA3 + u32(0x00). Rust writes only 0xA3. The client expects 5 bytes total.

8. send_update_tile_end — Wrong Wire Format
C++ sendUpdateTile 
protocolgame.cpp:2683-2703
:

Non-empty tile: GetTileDescription(tile, msg) + 0x00 + 0xFF
Empty tile: 0x01 + 0xFF
Rust send_update_tile_end:

Non-empty: 0x00 + 0xFF ✓ (but no tile description helper)
Empty: 0x01 + 0xFF ✓
The issue is the non-empty tile path — Rust doesn't have a full GetTileDescription equivalent in this function; it just writes end markers.

9. send_cancel_walk — Missing Direction
C++ sends 0xB5 + u8(player->getDirection()). Rust hardcodes 0xB5 + 0x00. The client will always show the character facing north after a cancel-walk.

10. Auto-Walk Path Direction — Reversed
C++ 
protocolgame.cpp:857-889
:

cpp
msg.skipBytes(numdirs);
for (uint8_t i = 0; i < numdirs; ++i) {
    uint8_t rawdir = msg.getPreviousByte();  // reads BACKWARDS
The C++ reads the path in reverse order (getPreviousByte). Rust reads it forward. Auto-walk paths will be reversed.

11. parseFightModes — Missing PVP Mode Byte
C++ 
protocolgame.cpp:1015-1031
 reads 3 bytes (fight, chase, secure) and comments out the PVP mode byte.

Rust reads 3 bytes — but the OTClient v8 for 1098 with GamePVPMode enabled sends 4 bytes (fight, chase, secure, pvp). The Rust parser doesn't consume the 4th byte, leaving it in the stream to corrupt the next packet.

12. parseAttack / parseFollow — C++ Only Reads One u32
C++ 
protocolgame.cpp:1034-1045
:

cpp
uint32_t creatureId = msg.get<uint32_t>();
// msg.get<uint32_t>(); creatureId (same as above)  // ← COMMENTED OUT
Rust reads two u32s (4+4=8 bytes). But the C++ source has the second read commented out — meaning the client only sends one u32. Rust will read 4 garbage bytes from the next packet.

WARNING

This will cause immediate desync on any attack or follow action, corrupting subsequent packets.

🟡 MEDIUM — Functional Gaps (Features Not Working)
13. sendCreatureSay — Different from sendChannelMessage
The Rust send_channel_message function is used for both channel and direct speech, but the C++ has separate functions:

sendCreatureSay (0xAA): statementId + name + u16(level) + type + position + text
sendToChannel (0xAA): statementId + name_or_null + u16(level) + type + u16(channelId) + text
sendPrivateMessage (0xAA): statementId + name + u16(level) + type + text
sendChannelMessage (0xAA): u32(0) + author + u16(0) + type + u16(channel) + text
Rust has one function that doesn't match any of these exactly.

14. Missing Server→Client Opcodes
These C++ send* functions have no Rust equivalent:

Opcode	C++ Function	Purpose
0x6A	sendAddTileItem	Item appears on tile
0x6B	sendUpdateTileItem	Item updated on tile
0x6E	sendContainer	Open container window
0x72	sendRemoveContainerItem	Remove item from container
0x78	sendInventoryItem (with item)	Equipped item in slot
0x7A	sendShop	NPC shop window
0x7B	sendSaleItemList	Sale prices list
0x96	sendTextWindow	Read/write text window
0xAC	sendChannel	Open channel with users
0xB4	sendTextMessage (damage variants)	Damage/heal/exp messages with position
0xC8	sendOutfitWindow (full)	Outfit selection with addon/mount lists
0xD2	sendVIP (full)	VIP entry with name/desc/icon/notify
0xF6	sendMarketEnter (full)	Market UI with depot items
0xF9	sendMarketBrowseItem	Market item offers
0xFA	sendModalWindow	Modal dialog window
0xBE	MoveUpCreature	Floor change up map strips
0xBF	MoveDownCreature	Floor change down map strips
15. Missing MoveUpCreature / MoveDownCreature
The Rust send_move_creature_player handles same-floor movement correctly but does not implement:

Floor change from surface to underground (oldPos.z == 7 && newPos.z >= 8)
MoveUpCreature (0xBE + additional floor descriptions)
MoveDownCreature (0xBF + additional floor descriptions)
The Rust code falls back to a full sendMapDescription for z-changes, which works but is more expensive and may cause visual artifacts (flash).

16. Creature Visibility Checks
C++ has extensive canSee / canSeeCreature checks before sending creature updates:

Ghost mode creatures
Invisible creatures (stealth ring)
isHealthHidden()
Rust creature_encode.rs has no such logic — all creatures are always visible to all players. Invisible or GM-ghost creatures will be visible to everyone.

17. Known Creature Eviction — Wrong Logic
C++ checkCreatureAsKnown 
protocolgame.cpp:744-776
: When knownCreatureSet > 1300, it specifically searches for a creature the player cannot see before evicting. If none found, it evicts the first that isn't the current creature.

Rust check_creature_known 
map_description.rs:36-47
: Simply evicts the first element from the HashSet iterator, without any visibility check.

🔵 LOW — Minor Differences / Cosmetic
18. send_relogin_window — Data Type Match ✓
Both C++ and Rust write 0x28 + 0x00 + u8(unfairFightReduction). ✓

19. send_quest_line — Field Order Difference
C++ sends 0xF1 + u16(questId) + u8(missions) then mission details. Rust sends 0xF1 + u16(questId) + u8(completed) + string(name). These are different data shapes but the Rust version appears to be a stub.

20. World Light — Hardcoded
Rust sends send_world_light(0, 215, false) — a constant dark level. C++ sends g_game.getWorldLightInfo() which varies with the day/night cycle.

21. Various Stubs
These Rust functions exist as stubs (minimal or placeholder data):

send_basic_data_stub / send_player_data_appear_stub — should not be used in production
send_modal_window_stub — incomplete packet
send_vip_entries_empty — always empty list
send_items_inventory_stub — hardcoded 11 sentinel items
send_combat_analyzer_stub — C++ also does nothing (return;)
Login Flow Comparison
C++ Login Sequence (sendAddCreature self branch, lines 2771-2825):
1. 0x17  SelfAppear (player id + beat + speed formula + flags + store)
2. 0x0A  PendingStateEntered
3. 0x43  OTCFeatures (if otclientV8)
4. 0x0F  EnterWorld
5. 0x64  MapDescription
6. 0x83  MagicEffect (TELEPORT)
7. 0x78× InventoryItem (slots 1-10 + StoreInbox)
8. 0xA0  PlayerStats
9. 0xB7  UnjustifiedStats  
10. 0x9F  BasicData
11. 0xA1  PlayerSkills
12. 0x82  WorldLight
13. 0x8D  CreatureLight (player)
14. 0xD2× VIPEntries
15. 0x9F  BasicData (again!)
16. 0xA2  PlayerIcons
Rust Login Sequence (
login_out.rs:332-382
):
1. 0x43  OTCv8 Features (extended opcode + tooltip)
2. 0x32  ExtendedOpcode (empty init)
3. 0x17  SelfAppear
4. 0x0A  PendingStateEntered
5. 0x43  OTC Features Raw (GameUnjustifiedPoints=68)  
6. 0x0F  EnterWorld
7. 0x64  MapDescription
8. 0x83  MagicEffect
9. 0x79× InventorySlotEmpty (1-11)
10. 0xA0  PlayerStats
11. 0xB7  UnjustifiedStats (stub)
12. 0x9F  BasicData
13. 0xA1  PlayerSkills
14. 0x82  WorldLight (hardcoded)
15. 0x8D  CreatureLight
16. 0xD2  VIP (empty)
17. 0x9F  BasicData (again)
18. 0xA2  Icons
19. 0xA7  FightModes
Differences:

Rust sends OTCv8 features before SelfAppear; C++ sends them after (inside login() before sendAddCreature)
Rust sends 0x43 twice (different feature sets) — one for OTCv8 features, one for GameUnjustifiedPoints
Rust sends FightModes at the end; C++ does not in sendAddCreature (it's sent elsewhere via sendFightModes)
Both send BasicData twice ✓ (matches C++ which calls sendBasicData() at line 2812 and 2823)
OTClient vs Official Client Detection
C++ Detection Points
┌─ Login Port (protocollogin.cpp:244-249) ────────────────────────┐
│ After account+password, read u16 length                         │
│ If length == 5 && getString(5) == "OTCv8" → otclientV8 = u16   │
└─────────────────────────────────────────────────────────────────┘
               ↓
┌─ Game Port (protocolgame.cpp:374-475) ──────────────────────────┐
│ OS = msg.get<u16>() → OperatingSystem_t                        │
│   - 0-9: Official/Linux/Windows/Mac                             │
│   - 10-12: CLIENTOS_OTCLIENT_LINUX/WINDOWS/MAC                 │
│   - 20-25: CLIENTOS_OTCLIENTV8_LINUX/.../WEB                   │
│                                                                  │
│ After auth, same OTCv8 string probe:                             │
│   u16 len == 5 && "OTCv8" → otclientV8 = version u16           │
│                                                                  │
│ Effects of detection:                                            │
│   1. otclientV8 → sendFeatures() on login                       │
│   2. OS >= OTCLIENT_LINUX → extended opcode init + register     │
│   3. otclientV8 → withDescription=true in addItem calls         │
│   4. otclientV8 → sendFeatures() with feature map               │
└─────────────────────────────────────────────────────────────────┘
Rust Detection: ❌ NONE
game_first_packet.rs parses OS u16 but discards it (no enum, no storage)
No OTCv8 string probe after challenge/auth
No operatingSystem stored on the connection
All connections treated as OTClient unconditionally
Item Encoding Deep Dive
TFS C++ addItem(const Item*, bool withDescription):
u16(clientId) + u8(0xFF MARK) + [stackable: u8(count)] + [fluid: u8(fluidMap)] 
+ [animation: u8(0xFE)] + [withDescription: string(desc)] + u8(duration_flag) 
+ [if duration: u32(duration) + u8(stopTime)]
TFS C++ addItem(uint16_t id, uint8_t count, bool withDescription) (template):
u16(clientId) + u8(0xFF MARK) + [stackable: u8(count)] + [fluid: u8(fluidMap)]
+ [animation: u8(0xFE)] + [withDescription: string("")] + u8(0x00 no-duration)
Rust write_item_template:
u16(clientId) + u8(0xFF MARK) + [stackable: u8(count)] + [animation: u8(0xFE)]
Missing from Rust:

❌ Fluid sub-type byte
❌ withDescription string (empty or not)
❌ Duration byte (0x00)
Recommended Fix Priority
#	Fix	Severity	Effort
1	Add duration 0x00 byte to write_item_template	🔴 Critical	1 line
2	Add fluid/splash sub-type byte to write_item_template	🔴 Critical	~10 lines
3	Fix parseAttack/parseFollow to read 1 u32, not 2	🔴 Critical	2 lines
4	Fix sendChangeSpeed to write 2×u16 not 1×u32	🔴 Critical	3 lines
5	Fix sendChannelMessage field order	🔴 Critical	5 lines
6	Rewrite send_creature_turn to match C++ format	🔴 Critical	15 lines
7	Fix auto-walk path to read backwards	🟠 High	5 lines
8	Parse and store OS / OTCv8 detection	🟠 High	30 lines
9	Add PVP mode byte to parseFightModes	🟠 High	2 lines
10	Fix sendCancelTarget to include u32(0)	🟠 High	1 line
11	Fix send_cancel_walk to accept direction	🟡 Medium	2 lines
12	Implement MoveUpCreature/MoveDownCreature	🟡 Medium	50 lines
13	Implement missing server opcodes (containers, shops, etc.)	🟡 Medium	200+ lines
14	Implement canSee/visibility filters	🟡 Medium	30 lines
15	Fix known creature eviction logic	🔵 Low	10 lines
16	Dynamic world light cycle	🔵 Low	5 lines
Files Audited
Rust (tfs-rust-net)
protocol_game.rs
 — XTEA framing
game_parse.rs
 — Client→server parsing
outgoing.rs
 — Basic server→client
outgoing_extra.rs
 — Extended server→client
map_description.rs
 — Map encoding
creature_encode.rs
 — Creature wire format
item_encode.rs
 — Item wire format
game_first_packet.rs
 — First packet parsing
game_challenge.rs
 — Challenge send
server.rs
 — Connection handling
message.rs
 — NetworkMessage type
Rust (tfs-rust-core)
login_out.rs
 — Login packet sequence
Rust (tfs-rust-common)
protocol_opcodes.rs
 — Opcode constants
protocol_constants.rs
 — Viewport constants
C++ (TFS 1.4.2)
protocolgame.cpp
 — Full protocol implementation
protocolgame.h
 — Protocol header
protocollogin.cpp
 — Login protocol
networkmessage.cpp
 — Wire format helpers
networkmessage.h
 — Wire format header
