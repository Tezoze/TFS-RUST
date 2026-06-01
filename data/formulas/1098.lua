-- data/formulas/1098.lua — TFS 1.4.2 / 10.98 mechanics defaults.
--
-- Tier-1 constants (loaded once into MechanicsProfile). Any key omitted falls back to the built-in
-- MechanicsProfile::for_version(1098) default, so this file is a readable mirror of those defaults —
-- edit a value to retune the shard without recompiling (docs/PROTOCOL_VERSIONING.md §12.13).
--
-- Behavior source: repo-root TFS 1.4.2 src/ (creature.cpp getStepDuration, map.cpp getPathMatching
-- fixed 10/25, weapons.cpp, condition.cpp, vocation.cpp).

formulas = {
  beatMs = 50,                  -- scheduler quantization (creature.cpp getStepDuration ceil to 50)
  attackSpeedMs = 0,            -- 0 = use vocation/weapon getAttackSpeed()
  defenseGateMs = 2000,
  armor = "full",               -- subtract full armor value
  pathCost = "fixed",           -- A* 10 normal / 25 diagonal
  weakestTargetMetric = "maxHp",
  distanceKeep = "perType",     -- per-MonsterType targetDistance
  damageFormula = "modern",
  spawnNearPlayer = "block",
  levelExp = "tfs",
  levelExpDelta = 100,            -- TFS getExpForLevel = (((L-6)*L+17)*L-12)/6 * 100
  expAttributionRounds = 60,

  fightModes = {
    offensiveAtk = 1.20, defensiveAtk = 0.80,
    offensiveDef = 0.80, defensiveDef = 1.20,
  },

  conditions = {
    fire   = { dmg = 10, ticks = 8 },
    energy = { dmg = 25, ticks = 10 },
    poisonStart = 50,
  },

  spell = { levelMult = 2, magicMult = 3 },
  pvpExpCap = { num = 11, den = 10 },
}

-- Tier-2 override hooks (optional). Omit to keep the native era-faithful default (zero runtime cost).
-- Example — uncomment to reshape weapon damage from Lua:
--
-- function getWeaponDamage(skill, attack, mode, level)
--   local maxv = attack * (skill * 5 + 50)
--   return math.floor(((math.random(0,99) + math.random(0,99)) / 2) * maxv / 10000)
-- end
