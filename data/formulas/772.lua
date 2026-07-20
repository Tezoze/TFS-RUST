-- data/formulas/772.lua — 772 mechanics defaults.
--
-- Tier-1 constants (loaded once into MechanicsProfile). Any key omitted falls back to the built-in
-- MechanicsProfile::for_version(772) default. Edit a value to retune the shard without recompiling
-- (docs/PROTOCOL_VERSIONING.md §12.13).


formulas = {
  beatMs = 50,                    -- 772 `.tibia` config sets Beat=50 (config.cc:187 overrides default 200)
  stepBeatMs = 50,                -- TVP gameserver quantizer (wire reference); beat loop uses beatMs
  defenseGateMs = 2000,
  armor = "randomized",         -- (Armor/2) + rand%(Armor/2)
  pathCost = "terrain",         -- terrain-speed-weighted waypoints, diagonal 3x
  pathSearch = "reverse",     -- reverse TShortway dest→origin; 1098 uses "forward"
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
  -- Per-skill tries curve (PC-5): human.mon Delta/Min; distinct from damageTuning.skillBase.
  skillTuning = {
    skillBase = {50, 50, 50, 50, 30, 100, 20},  -- fist, club, sword, axe, dist, shielding, fishing
    minLevel  = {10, 10, 10, 10, 10, 10, 10},
    magicSkillBase = 1600,
    magicMinLevel = 0,
  },
  spawnNearPlayer = "shrink",   -- radius shrink near players, still spawn
  spawnPlacement = "classic772", -- SearchSpawnField BFS (monster.db homes)
  respawnModel = "monsterhome772", -- StartMonsterhomeTimer: random(regen/2,regen) + crowd scaling
  expAttributionRounds = 60,
  followRepathWithoutPath = true,  -- target-move repath without hasFollowPath gate (not idle drain)
  pathForwardFallback = false,     -- NOWAY when reverse search fails
  corpseDecayOffsetMs = 30000,     -- generic corpse decay +30s (crmain.cc decay scheduler)
  classicEquipmentSlots = true,      -- 772 hand slots accept any pickupable item
  undergroundSeesSurface = true,   -- IsVisible: underground CAN see surface ±2 floors
  damageTextFormat = "attackerAttribution", -- "You lose N hp due to an attack by X."

  fightModes = {
    offensiveAtk = 1.20, defensiveAtk = 0.60,
    offensiveDef = 0.60, defensiveDef = 1.80,
  },

  conditions = {
    fire   = { dmg = 10, ticks = 8 },
    energy = { dmg = 25, ticks = 10 },
    poisonStart = 50,
  },

  spell = { levelMult = 2, magicMult = 3 },  -- ComputeDamage; Player:computeDamage reads these
  pvpExpCap = { num = 11, den = 10 },
  playerSpeed = "772",      -- "772" | "retail" | "balanced" (loaded once at startup)
}

-- Player speed model selector ------------------------------------------------------------
--
-- Controls how walk speed scales with level. Set formulas.playerSpeed to one of:
--
--   "772"      classic 772 linear formula (base = vocation_base + (level-1),
--              eff = 2*base + 80). vocation_base=70 from `human.mon` GoStrength
--              (decompile `crskill.cc:667` `TSkillAdd::Advance`, AddLevel=1).
--              Breakpoints at base=70: 250ms@190, 200ms@265, 150ms@390.
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
