-- data/formulas/772.lua — CipSoft-faithful 7.72 mechanics defaults.
--
-- Tier-1 constants (loaded once into MechanicsProfile). Any key omitted falls back to the built-in
-- MechanicsProfile::for_version(772) default. Edit a value to retune the shard without recompiling
-- (docs/PROTOCOL_VERSIONING.md §12.13).
--
-- Behavior source (clean-room outcomes, R12): tibia-game-master/src/
--   beat 200            -> config.cc Beat = 200
--   speed*2+80          -> crmain.cc:445 TCreature::GetSpeed
--   step ceil to Beat   -> cract.cc:1462 TCreature::NotifyGo
--   terrain path        -> cract.cc TShortway (diagonal 3x tile cost)
--   attack 2000 ms      -> crcombat.cc:145 TCombat::DelayAttack(2000)
--   randomized armor    -> crcombat.cc:303 (A/2)+rand(A/2)
--   fight modes         -> crcombat.cc:222 (+20% atk / -40% def offensive; -40% atk / +80% def defensive)
--   fire 10/8 nrg 25/10 -> crskill.cc:1064 / :1090
--   spell 2*lvl+3*ml    -> magic.cc:784 ComputeDamage
--   level exp poly      -> crskill.cc:352 TSkillLevel::GetExpForLevel

formulas = {
  beatMs = 200,
  stepBeatMs = 50,              -- TVP walk quantizer (`gameserver/src/creature.cpp`), not CipSoft Beat 200
  -- stepSpeedModel = "cipsoft",  -- default: GoStrength*2+80 linear delay
  attackSpeedMs = 2000,         -- flat swing, not weapon speed
  defenseGateMs = 2000,
  armor = "randomized",         -- (Armor/2) + rand%(Armor/2)
  pathCost = "terrain",         -- terrain-speed-weighted waypoints, diagonal 3x
  weakestTargetMetric = "currentHp",
  distanceKeep = 4,             -- hardcoded range 4 (crnonpl.cc:2716)
  damageFormula = "classic",    -- ProbeValue
  spawnNearPlayer = "shrink",   -- radius shrink near players, still spawn
  levelExp = "cipsoft",
  levelExpDelta = 100,
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
}

-- Tier-2 override hooks (optional). The native defaults already reproduce the CipSoft outcome, so
-- leave these unset for a faithful 7.72 shard. Example:
--
-- function getWeaponDamage(skill, attack, mode, level)
--   local maxv = attack * (skill * 5 + 50)
--   return math.floor(((math.random(0,99) + math.random(0,99)) / 2) * maxv / 10000)
-- end
