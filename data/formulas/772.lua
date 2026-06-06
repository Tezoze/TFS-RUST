-- data/formulas/772.lua — CipSoft-faithful 7.72 mechanics defaults.
--
-- Tier-1 constants (loaded once into MechanicsProfile). Any key omitted falls back to the built-in
-- MechanicsProfile::for_version(772) default. Edit a value to retune the shard without recompiling
-- (docs/PROTOCOL_VERSIONING.md §12.13).


formulas = {
  beatMs = 200,
  stepBeatMs = 50,               -- TVP walk quantizer (`gameserver/src/creature.cpp`), not CipSoft Beat 200
  defenseGateMs = 2000,
  armor = "randomized",         -- (Armor/2) + rand%(Armor/2)
  pathCost = "terrain",         -- terrain-speed-weighted waypoints, diagonal 3x
  distanceKeep = "perType",     -- keep band from each monster's XML targetDistance
  weakestTargetMetric = "currentHp",
  damageFormula = "classic",    -- ProbeValue
  damageTuning = {
    skillMult = 5,
    skillBase = 50,
    randomMax = 99,
  },
  armorTuning = {
    minArmorForRandom = 2,
    divisor = 2,
  },
  spawnNearPlayer = "shrink",   -- radius shrink near players, still spawn
  expAttributionRounds = 60,

  fightModes = {
    offensiveAtk = 1.20, defensiveAtk = 0.60,
    offensiveDef = 0.60, defensiveDef = 1.80,
  },

  conditions = {
    fire   = { dmg = 10, ticks = 8 },
    energy = { dmg = 25, ticks = 10 },
    poisonStart = 50,
  },

  spell = { levelMult = 2, magicMult = 3 },
  pvpExpCap = { num = 11, den = 10 },
  playerSpeed = "balanced",      -- "772" | "retail" | "balanced" (loaded once at startup)
}

-- Player speed model selector ------------------------------------------------------------
--
-- Controls how walk speed scales with level. Set formulas.playerSpeed to one of:
--
--   "772"      classic 772 linear formula (base = 220 + level, eff = 2*base + 80).
--              Gets very fast at high levels — breakpoints: 250ms@39, 200ms@114, 150ms@237.
--
--   "retail"   1098/TFS logarithmic formula (floor(857.36 * ln(base/2 + 261.29) - 4795.01)).
--              Slower at low levels, never reaches 150ms in normal level ranges.
--
--   "balanced" Logarithmic diminishing-returns curve anchored to classic 772 feel up to ~100,
--              then softened. Keeps the old-school speed tier feel without the "blink across
--              screen" problem at high levels. 150ms delayed to ~level 453, 100ms unreachable.
--              (See comparison: docs/PROTOCOL_VERSIONING.md §12.13)
--
-- Runtime note: playerSpeed / damageTuning / armorTuning are loaded once at startup into Rust
-- `MechanicsProfile` and then run natively in the game loop (no per-step Lua callback overhead).
