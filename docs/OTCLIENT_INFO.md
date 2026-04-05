# OTClient v8 — `0xB7` unjustified stats & `0xA1` player skills (1098)

Source of truth: `src/client/protocolgameparse.cpp`, `src/client/protocolcodes.h`, `src/client/game.h`, `modules/game_features/features.lua`.

Opcode constants (`protocolcodes.h`):

- `GameServerUnjustifiedStats = 183` (`0xB7`)
- `GameServerPlayerSkills = 161` (`0xA1`)
- `GameServerPlayerModes = 167` (`0xA7`)

---

## 1. `0xB7` — `parseUnjustifiedStats`

**Dispatch:** `case Proto::GameServerUnjustifiedStats:` → `parseUnjustifiedStats(msg);` — **no** `getFeature` guard on the switch; the handler always runs and **always reads the same bytes**.

### Exact read order (payload after opcode)

| # | Read | Field |
|---|------|--------|
| 1 | `getU8()` | `killsDay` |
| 2 | `getU8()` | `killsDayRemaining` |
| 3 | `getU8()` | `killsWeek` |
| 4 | `getU8()` | `killsWeekRemaining` |
| 5 | `getU8()` | `killsMonth` |
| 6 | `getU8()` | `killsMonthRemaining` |
| 7 | `getU8()` | `skullTime` |

**Total payload: 7 bytes** (all `u8`). No `u16`/`u32`, no strings, no loops.

### Feature guards

- **Parser:** none — bytes are consumed even if `GameUnjustifiedPoints` is off.
- **`g_game.setUnjustifiedPoints`:** in `game.cpp`, if `!getFeature(Otc::GameUnjustifiedPoints)` the update is **skipped** (no Lua event), but the **network buffer was already advanced** by `parseUnjustifiedStats`.

### Alignment vs common server stubs

- **`send_unjustified_stats_stub` with 8× `0x00`:** client reads **7** bytes → **one byte remains** in the submessage for this opcode → typical **“1 unread” / drift** unless the extra byte is not actually sent as part of this packet.
- **Bundled TFS-style 7 bytes after `0xB7`:** matches this client **exactly**.

### Full C++ (copy-paste)

```944:956:otclientv8-master/src/client/protocolgameparse.cpp
void ProtocolGame::parseUnjustifiedStats(const InputMessagePtr& msg)
{
    UnjustifiedPoints unjustifiedPoints;
    unjustifiedPoints.killsDay = msg->getU8();
    unjustifiedPoints.killsDayRemaining = msg->getU8();
    unjustifiedPoints.killsWeek = msg->getU8();
    unjustifiedPoints.killsWeekRemaining = msg->getU8();
    unjustifiedPoints.killsMonth = msg->getU8();
    unjustifiedPoints.killsMonthRemaining = msg->getU8();
    unjustifiedPoints.skullTime = msg->getU8();

    g_game.setUnjustifiedPoints(unjustifiedPoints);
}
```

Struct (`game.h`):

```39:56:otclientv8-master/src/client/game.h
struct UnjustifiedPoints {
    bool operator==(const UnjustifiedPoints& other) {
        return killsDay == other.killsDay &&
            killsDayRemaining == other.killsDayRemaining &&
            killsWeek == other.killsWeek &&
            killsWeekRemaining == other.killsWeekRemaining &&
            killsMonth == other.killsMonth &&
            killsMonthRemaining == other.killsMonthRemaining &&
            skullTime == other.skullTime;
    }
    uint8 killsDay;
    uint8 killsDayRemaining;
    uint8 killsWeek;
    uint8 killsWeekRemaining;
    uint8 killsMonth;
    uint8 killsMonthRemaining;
    uint8 skullTime;
};
```

---

## 2. `0xA1` — `parsePlayerSkills`

### preamble: how many skill slots

- `lastSkill = Otc::Fishing + 1` → **7** skills (indices `0..6`: Fist … Fishing) if **`GameAdditionalSkills`** is **off**.
- `lastSkill = Otc::LastSkill` → **13** skills (indices `0..12`: adds Critical / Life leech / Mana leech fields) if **`GameAdditionalSkills`** is **on**.

`Otc::Skill` (`const.h`): `Fist` … `Fishing` = 0..6; `CriticalChance` … `ManaLeechAmount` = 7..12; `LastSkill` = 13.

### Tibia 12 block (not used for 1098 stock)

If `GameTibia12Protocol` (**enabled only for `version >= 1200`** in `features.lua`):

- Before the skill loop: `getU16` level, `getU16` base, `getU16` unknown, `getU16` levelPercent for **magic** (then loop skips magic in loop — see source).
- After the loop: `getU32` total capacity, `getU32` base capacity.

For **protocol 1098** with default features, **`GameTibia12Protocol` is off** → **none** of these reads run.

### Per-skill loop (1098-relevant)

For each `skill` in `0 .. lastSkill-1`:

1. **Level**
   - `GameDoubleSkills` (`>= 1035`): `getU16()`
   - else: `getU8()`

2. **Base level**
   - If `GameSkillsBase` (`>= 910`):
     - If `GameBaseSkillU16` (`>= 1035`): `getU16()`
     - else: `getU8()`
   - else: `baseLevel = level` (no read)

3. **Level percent** — only for **`skill <= Otc::Fishing`** (core 7 skills):
   - If `GameTibia12Protocol`: `getU16()` unknown, then `getU16()` percent  
   - else: `getU8()` percent  
   For **additional skills** (7–12): **no** percent bytes (comment: critical / leech have no percent).

### Byte budget sketch for **1098** (default `features.lua`)

Assume: `GameDoubleSkills`, `GameSkillsBase`, `GameBaseSkillU16` **on**; `GameTibia12Protocol` **off**; `GameAdditionalSkills` **on** (`1098 >= 1094`).

- **7 core skills** (0–6): per skill `u16` + `u16` + `u8` = **5 bytes** → **35 bytes**
- **6 additional skills** (7–12): per skill `u16` + `u16` = **4 bytes** → **24 bytes**
- **Total ≈ 59 bytes** after opcode (no Tibia12 magic header/footer).

If the server sends the **old 7-skill / 35-byte** layout but the client has **`GameAdditionalSkills`** enabled, the client will **read 24 more bytes** from the stream → **EOF or wrong next opcode** unless the server extends the packet.

If you see **“1 unread”** after `0xA1`, typical causes:

- Server sent **one fewer** byte than the client expected (e.g. missing one `u8` percent on a core skill, or missing an entire extra-skill block while client has `GameAdditionalSkills`).
- Or **one extra** byte on server while client expected different width (e.g. `u8` level vs `u16` with `GameDoubleSkills` off on one side).

### Full C++ (copy-paste)

```2011:2064:otclientv8-master/src/client/protocolgameparse.cpp
void ProtocolGame::parsePlayerSkills(const InputMessagePtr& msg)
{
    int lastSkill = Otc::Fishing + 1;
    if (g_game.getFeature(Otc::GameAdditionalSkills))
        lastSkill = Otc::LastSkill;

    if (g_game.getFeature(Otc::GameTibia12Protocol)) {
        int level = msg->getU16();
        int baseLevel = msg->getU16();
        msg->getU16(); // unknown
        int levelPercent = msg->getU16();
        m_localPlayer->setMagicLevel(level, levelPercent);
        m_localPlayer->setBaseMagicLevel(baseLevel);
    }

    for (int skill = 0; skill < lastSkill; skill++) {
        int level;

        if (g_game.getFeature(Otc::GameDoubleSkills))
            level = msg->getU16();
        else
            level = msg->getU8();

        int baseLevel;
        if (g_game.getFeature(Otc::GameSkillsBase))
            if (g_game.getFeature(Otc::GameBaseSkillU16))
                baseLevel = msg->getU16();
            else
                baseLevel = msg->getU8();
        else
            baseLevel = level;

        int levelPercent = 0;
        // Critical, Life Leech and Mana Leech have no level percent
        if (skill <= Otc::Fishing) {
            if (g_game.getFeature(Otc::GameTibia12Protocol))
                msg->getU16(); // unknown

            if (g_game.getFeature(Otc::GameTibia12Protocol))
                levelPercent = msg->getU16();
            else
                levelPercent = msg->getU8();
        }

        m_localPlayer->setSkill((Otc::Skill)skill, level, levelPercent);
        m_localPlayer->setBaseSkill((Otc::Skill)skill, baseLevel);
    }

    if (g_game.getFeature(Otc::GameTibia12Protocol)) {
        uint32_t totalCapacity = msg->getU32();
        msg->getU32(); // base capacity?
        m_localPlayer->setTotalCapacity(totalCapacity);
    }
}
```

**Note:** There is **no** separate `GameLeechAmount` feature in this tree — leech **slots** are included when `GameAdditionalSkills` is on (extra enum skills 7–12).

---

## 3. `features.lua` — what applies to **version 1098**

All conditions `version >= X` where `1098 >= X` are **true**. Relevant for unjustified / skills / PVP:

| Feature | Enabled at | For 1098 |
|---------|------------|----------|
| `GameUnjustifiedPoints` | ≥ 1053 | **yes** |
| `GameAdditionalSkills` | ≥ 1094 | **yes** |
| `GameDoubleSkills` | ≥ 1035 | **yes** |
| `GameBaseSkillU16` | ≥ 1035 | **yes** |
| `GameSkillsBase` | ≥ 910 | **yes** |
| `GamePVPMode` | ≥ 1000 | **yes** |
| `GameTibia12Protocol` | ≥ 1200 | **no** |

So for **1098**, `parsePlayerSkills` uses **13 skills**, **u16** level, **u16** base, **u8** percent for skills **0–6** only, unless overridden by server `GameServerFeatures` / `entergame` params / custom Lua.

Complete list of `enableFeature` lines that fire for **1098** (same as “all blocks from 770 through 1094” plus `GameBot`): copy from `modules/game_features/features.lua` lines **16–186** (every block whose threshold is ≤ 1098, stopping before `if(version >= 1100)`).

---

## 4. Optional — `0xA7` — `parsePlayerModes`

```2089:2100:otclientv8-master/src/client/protocolgameparse.cpp
void ProtocolGame::parsePlayerModes(const InputMessagePtr& msg)
{
    int fightMode = msg->getU8();
    int chaseMode = msg->getU8();
    bool safeMode = msg->getU8();

    int pvpMode = 0;
    if (g_game.getFeature(Otc::GamePVPMode))
        pvpMode = msg->getU8();

    g_game.processPlayerModes((Otc::FightModes)fightMode, (Otc::ChaseModes)chaseMode, safeMode, (Otc::PVPModes)pvpMode);
}
```

For **1098**, `GamePVPMode` is on (`>= 1000`) → **4 × `getU8()`** (fight, chase, safe, pvp). If `GamePVPMode` were off, only **3 bytes** would be read — mismatch with a server that always sends 4.

---

## Quick reference

| Opcode | Handler | Payload (this client) |
|--------|---------|------------------------|
| `0xB7` | `parseUnjustifiedStats` | **7 × u8** |
| `0xA1` | `parsePlayerSkills` | Depends on `GameAdditionalSkills`, `GameDoubleSkills`, `GameSkillsBase`, `GameBaseSkillU16`, `GameTibia12Protocol` (see §2) |
| `0xA7` | `parsePlayerModes` | **3 × u8**, + **1 × u8** if `GamePVPMode` (1098: **4 bytes**) |
