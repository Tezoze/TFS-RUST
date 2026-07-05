-- data/vocations.lua
-- Vocation combat data — Lua-as-data pilot (DATA_FORMAT_MIGRATION.md Phase 1).
-- Replaces data/XML/vocations.xml as the source of truth for the full TVP
-- combat block: per-level gains, regen cadence, formulas, skill multipliers,
-- soul, attack/base speed, and the level-1 vitals floor.
--
-- C++ reference (772 outcomes — tibia-game-master/src/):
--   * Per-vocation AddLevel for HP/mana/cap — crplayer.cc:1050-1093 SetProfession.
--   * Regen cadence (gain_hp_ticks/gain_hp_amount/…) — crskill.cc:828-885 TSkillFed::Event.
--   * Level-1 vitals floor (base_hp=150, base_mana=0, base_cap=400) —
--     runtime/mon/human.mon Skills = { (HitPoints, 150, 0, 150, …),
--     (Mana, 0, 0, 0, …), (CarryStrength, 400, 0, 400, …) } (race data;
--     AddLevel overrides only the per-level gain, not the floor).
--   * Skill multipliers feed TSkillProbe::GetExpForLevel FactorPercent
--     (crskill.cc:472-512).
--
-- skill_multipliers indices: 0=fist, 1=club, 2=sword, 3=axe, 4=distance,
-- 5=shielding, 6=fishing (matches SKILL_FIST..SKILL_FISHING in enums.hh).

local SKILL_FIST, SKILL_CLUB, SKILL_SWORD, SKILL_AXE = 0, 1, 2, 3
local SKILL_DISTANCE, SKILL_SHIELDING, SKILL_FISHING = 4, 5, 6

return {
  schema = 1,
  vocations = {
    {
      id = 0, client_id = 0, name = "None", description = "none",
      from_vocation = 0,
      gain_cap = 10, gain_hp = 5, gain_mana = 5,
      gain_hp_ticks = 6, gain_hp_amount = 1,
      gain_mana_ticks = 6, gain_mana_amount = 1,
      mana_multiplier = 4.0, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 100, gain_soul_ticks = 120, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1 },
    },
    {
      id = 1, client_id = 3, name = "Sorcerer", description = "a sorcerer",
      from_vocation = 1,
      gain_cap = 10, gain_hp = 5, gain_mana = 30,
      gain_hp_ticks = 12, gain_hp_amount = 1,
      gain_mana_ticks = 3, gain_mana_amount = 2,
      mana_multiplier = 1.1, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 100, gain_soul_ticks = 120, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1 },
    },
    {
      id = 2, client_id = 4, name = "Druid", description = "a druid",
      from_vocation = 2,
      gain_cap = 10, gain_hp = 5, gain_mana = 30,
      gain_hp_ticks = 12, gain_hp_amount = 1,
      gain_mana_ticks = 3, gain_mana_amount = 2,
      mana_multiplier = 1.1, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 100, gain_soul_ticks = 120, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.5, 1.8, 1.8, 1.8, 1.8, 1.5, 1.1 },
    },
    {
      id = 3, client_id = 2, name = "Paladin", description = "a paladin",
      from_vocation = 3,
      gain_cap = 20, gain_hp = 10, gain_mana = 15,
      gain_hp_ticks = 8, gain_hp_amount = 1,
      gain_mana_ticks = 4, gain_mana_amount = 2,
      mana_multiplier = 1.4, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 100, gain_soul_ticks = 120, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.2, 1.2, 1.2, 1.2, 1.1, 1.1, 1.1 },
    },
    {
      id = 4, client_id = 1, name = "Knight", description = "a knight",
      from_vocation = 4,
      gain_cap = 25, gain_hp = 15, gain_mana = 5,
      gain_hp_ticks = 6, gain_hp_amount = 1,
      gain_mana_ticks = 6, gain_mana_amount = 2,
      mana_multiplier = 3.0, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 100, gain_soul_ticks = 120, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.1, 1.1, 1.1, 1.1, 1.4, 1.1, 1.1 },
    },
    {
      id = 5, client_id = 3, name = "Master Sorcerer", description = "a master sorcerer",
      from_vocation = 1,
      gain_cap = 10, gain_hp = 5, gain_mana = 30,
      gain_hp_ticks = 12, gain_hp_amount = 1,
      gain_mana_ticks = 2, gain_mana_amount = 2,
      mana_multiplier = 1.1, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 200, gain_soul_ticks = 15, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1 },
    },
    {
      id = 6, client_id = 4, name = "Elder Druid", description = "an elder druid",
      from_vocation = 2,
      gain_cap = 10, gain_hp = 5, gain_mana = 30,
      gain_hp_ticks = 12, gain_hp_amount = 1,
      gain_mana_ticks = 2, gain_mana_amount = 2,
      mana_multiplier = 1.1, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 200, gain_soul_ticks = 15, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.5, 1.8, 1.8, 1.8, 1.8, 1.5, 1.1 },
    },
    {
      id = 7, client_id = 2, name = "Royal Paladin", description = "a royal paladin",
      from_vocation = 3,
      gain_cap = 20, gain_hp = 10, gain_mana = 15,
      gain_hp_ticks = 6, gain_hp_amount = 1,
      gain_mana_ticks = 3, gain_mana_amount = 2,
      mana_multiplier = 1.4, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 200, gain_soul_ticks = 15, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.2, 1.2, 1.2, 1.2, 1.1, 1.1, 1.1 },
    },
    {
      id = 8, client_id = 1, name = "Elite Knight", description = "an elite knight",
      from_vocation = 4,
      gain_cap = 25, gain_hp = 15, gain_mana = 5,
      gain_hp_ticks = 4, gain_hp_amount = 1,
      gain_mana_ticks = 6, gain_mana_amount = 2,
      mana_multiplier = 3.0, attack_speed_ms = 2000, base_speed = 70,
      soul_max = 200, gain_soul_ticks = 15, allow_pvp = false,
      base_hp = 150, base_mana = 0, base_cap = 400,
      formula = { melee_damage = 1.0, dist_damage = 1.0, defense = 1.0, armor = 1.0 },
      skill_multipliers = { 1.1, 1.1, 1.1, 1.1, 1.4, 1.1, 1.1 },
    },
  },
}
