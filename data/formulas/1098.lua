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
  defenseGateMs = 2000,
  armor = "full",               -- subtract full armor value
  pathCost = "fixed",           -- A* 10 normal / 25 diagonal
  weakestTargetMetric = "maxHp",
  damageFormula = "modern",
  damageTuning = {
    skillMult = 5,
    skillBase = 50,
    randomMax = 99,
  },
  armorTuning = {
    minArmorForRandom = 2,
    divisor = 2,
  },
  spawnNearPlayer = "block",
  expAttributionRounds = 60,
  followRepathWithoutPath = false,  -- TFS creature.cpp:619 requires hasFollowPath
  pathForwardFallback = true,      -- TFS falls back to forward search if reverse fails

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
  playerSpeed = "retail",        -- "retail" | "772" | "balanced" (loaded once at startup)
}

-- Player speed model selector ------------------------------------------------------------
--
-- 1098 native step timing uses the TfsLog model in Rust (floor(857.36*ln(base/2+261.29)-4795.01))
-- and does NOT call getCreatureSpeed() for the walk timer — it uses GoStrength directly.
-- Setting playerSpeed = "retail" here is the default no-op (native TfsLog path, zero extra cost).
--
-- Set formulas.playerSpeed = "balanced" to apply the same diminishing-returns curve as 772 "balanced",
-- useful if you want to share a speed feel across both eras.
-- Set formulas.playerSpeed = "772" to force classic CipSoft linear speed on a 1098 shard (unusual).

-- Runtime note: playerSpeed / damageTuning / armorTuning are loaded once at startup into Rust
-- `MechanicsProfile` and then run natively in the game loop (no per-step Lua callback overhead).

-- Tier-2 override hooks (optional). Omit to keep the native era-faithful default (zero runtime cost).
-- Example — uncomment to reshape weapon damage from Lua:
--
--[[
function getWeaponDamage(skill, attack, mode, level)
  local maxv = attack * (skill * 5 + 50)
  return math.floor(((math.random(0,99) + math.random(0,99)) / 2) * maxv / 10000)
end
]]
