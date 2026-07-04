# 772 Splash/Pool Layer Mismatch

The OTB `FLAG_ALWAYSONTOP` on splash items (group 11) places them in the wrong tile
layer relative to 772 `objects.srv`. This blocks blood-on-ladders parity and forces
`create_liquid_splash_772` to use a TFS-style "no splash in ladders" guard instead of
the decompile's `CreatePool` algorithm.

## Summary

| Item type | 772 `objects.srv` flags | Our OTB flags | Tile vector | Match? |
|---|---|---|---|---|
| **Pools/splashes** (group 11) | `BOTTOM` (below creatures) | `FLAG_ALWAYSONTOP` (above creatures) | `top_items` | **NO** — opposite layer |
| **Fluid containers** (group 12) | `LiquidContainer`, `Take` | NOT alwaysOnTop, `FLAG_PICKUPABLE` | inventory / `down_items` | Yes |
| **Liquid sources** (water tiles) | `Bank`, `LiquidSource`, `Unmove` | group 1 (GROUND) | `ground` | Yes |

Only splashes are mismatched. Fluid containers and liquid sources are correct.

## Evidence

### 772 `objects.srv` — pools are `BOTTOM`

`reference/cipsoft-772/runtime/dat/objects.srv`:

```
TypeID      = 2886
Name        = "a pool"
Flags       = {Bottom,LiquidPool,Unmove,Expire,Special}
Attributes  = {ExpireTarget=2887,TotalExpireTime=120,Meaning=31}

TypeID      = 2889
Name        = "a pool"
Flags       = {Bottom,LiquidPool,Unmove,Expire,Special}
Attributes  = {ExpireTarget=2890,TotalExpireTime=120,Meaning=30}
```

`enum FLAG` (`enums.hh:206`): `BOTTOM = 2` (below creatures), `TOP = 3` (above
creatures). Pools carry `BOTTOM`; ladders/signs carry `TOP`. They are on different
layers and coexist on the same tile without conflict.

### Our OTB — splashes are `FLAG_ALWAYSONTOP`, order 2

`data/items/items.otb` (parsed):

```
sid=2016 cid=2886 aot=True top_order=2 flags=0x00002000
sid=2017 cid=2887 aot=True top_order=2 flags=0x00002000
sid=2019 cid=2889 aot=True top_order=2 flags=0x00002000
...
```

`FLAG_ALWAYSONTOP` = bit 13 (`0x2000`). Ladders/signs/borders are also `alwaysOnTop`
with `alwaysOnTopOrder = 2`. Both route to `top_items` — same layer, same order.

### Decompiled `CreatePool` scans `BOTTOM` only

`reference/cipsoft-772/tibia-game-master/src/operate.cc:2585-2619`:

```cpp
void CreatePool(Object Con, ObjectType Type, uint32 Value){
    // ...
    Object Help = GetFirstContainerObject(Con);
    while(Help != NONE){
        Object Next = Help.getNextObject();
        ObjectType HelpType = Help.getObjectType();
        if(HelpType.getFlag(BOTTOM)){
            if(!HelpType.getFlag(LIQUIDPOOL)){
                throw NOROOM;          // non-pool BOTTOM object blocks
            }
            try{
                Delete(Help, -1);      // existing pool → replace
            }catch(RESULT r){ ... }
        }
        Help = Next;                   // TOP objects (ladders) skipped
    }
    Create(Con, Type, Value);
}
```

Callers catch `NOROOM` silently:
- Death pool: `crmain.cc:218-226` (`~TCreature`)
- Hit splash: `crmain.cc:766-774` (`TCreature::Damage` physical branch)

## Observable consequence

| Scenario | 772 decompile | Our current code | Result |
|---|---|---|---|
| Pool + ladder | OK (different layers) | Blocked (`NOROOM` equivalent) | **Opposite** |
| Pool + corpse | `NOROOM` (same `BOTTOM` layer) | OK (corpse in `down_items`, pool in `top_items`) | **Opposite** |
| Pool + old pool | Delete old, place new | Delete old, place new | Same |

The ladder case is the visible one: 772 allows blood splashes on ladder tiles; we don't.

## Why we can't just match the decompile literally

Our OTB puts splashes and ladders in the same `top_items` vector with
`alwaysOnTopOrder = 2`. Placing both on the same tile produces two order-2 top items.
The 772 client's `0x6A` (add tile item) omits stackpos — the client places the item by
`.dat` `alwaysOnTopOrder`, inserting equal-order items **before** existing ones. If our
server vector order doesn't match, `0x6C` remove hits the wrong client stackpos.

## Option A (runtime override) — REJECTED (2026-07-04, verified by live client test)

Moving splashes to `down_items` (clearing `FLAG_ALWAYSONTOP` at OTB load) desyncs the
client: the 772 client's `0x6A` add omits stackpos, so the client places the splash in
`top_items` based on `.dat` (which still says `alwaysOnTop`), but the server's `0x6C`
remove uses the `down_items` stackpos → mismatch → client deletes the wrong item (ladder
or player). The `.dat` file cannot be changed at runtime. See
`reference/tvp-772/gameserver/src/protocolgame.cpp:1591-1605` (`sendAddTileItem` — no
stackpos for real client).

## Fix: Sorted top-item insertion (2026-07-04, verified by live client test)

The real bug was that `add_top_item` did `push` (append to end), but TVP's
`Tile::addThing` (`tile.cpp:898-906`) inserts **sorted by `alwaysOnTopOrder`**:

```cpp
if (itemType.alwaysOnTopOrder <= Item::items[(*it)->getID()].alwaysOnTopOrder) {
    items->insert(it, item);  // insert BEFORE existing
}
```

Splash and ladder both have `alwaysOnTopOrder = 2`. TVP inserts the splash **before**
the ladder. Our code appended it **after**. The 772 client sorts by `.dat`
`alwaysOnTopOrder` on `0x6A` (no stackpos), so it placed the splash before the ladder —
but our `0x6C` remove used the server's stackpos (splash after ladder) → client removed
the ladder instead.

### Changes

- `crates/tfs-rust-core/src/tile.rs` — added `add_top_item_at(index)` for sorted insertion.
- `crates/tfs-rust-core/src/game_world_item_cylinder.rs` — `internal_add_item_to_tile`
  now computes the sorted insertion index (first position where
  `new_order <= existing_order`) and uses `add_top_item_at`, matching TVP
  `Tile::addThing` (`tile.cpp:901`).

The server's `top_items` vector order now matches the client's `.dat`-based order, so
`get_item_stack_pos` returns the same index the client uses. Blood on ladders works;
splashes and ladders coexist in `top_items` (decompile `CreatePool` behavior — different
server-side layers, same wire layer).

## Related

- `docs/772_OTB_OBJECTS_SRV_FLAG_MAPPING.md` — full flag/group mapping table
- `crates/tfs-rust-core/src/creature/monster_inventory.rs` — `create_liquid_splash_772`
- `reference/cipsoft-772/tibia-game-master/src/operate.cc:2585-2619` — `CreatePool`
- `reference/cipsoft-772/tibia-game-master/src/crmain.cc:205-227, 706-775` — call sites
- `reference/tvp-772/gameserver/src/tile.cpp:898-906` — sorted top-item insertion
- `reference/tvp-772/gameserver/src/protocolgame.cpp:1591-1605` — `sendAddTileItem` (no stackpos)
