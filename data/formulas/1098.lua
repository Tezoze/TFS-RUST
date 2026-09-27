-- Adjustable 1098 mechanics. Omitted keys keep the built-in profile.

formulas = {
  defenseGateMs = 2000, -- minimum time between defense rolls
  armor = "full", -- "randomized" rolls half the armor, "full" subtracts it
  distanceKeep = "perType", -- "perType" uses each monster's targetDistance; a number forces one distance
  damageFormula = "modern", -- "classic" is the probe roll, "modern" is the level-and-skill formula
  -- Weapon max is attack * (skill * skillMult + skillBase). Each die rolls 0..randomMax.
  damageTuning = {
    skillMult = 5,
    skillBase = 50,
    randomMax = 99,
  },
  -- Used when armor is "randomized" and armor >= minArmorForRandom: (armor / divisor) plus a roll of that size.
  armorTuning = {
    minArmorForRandom = 2,
    divisor = 2,
  },
  -- Multipliers for offensive and defensive stance. Balanced stance stays at 1.
  fightModes = {
    offensiveAtk = 1.20, defensiveAtk = 0.80,
    offensiveDef = 0.80, defensiveDef = 1.20,
  },
  expAttributionRounds = 60, -- combat rounds that still count toward kill experience
  combatListSlots = 20, -- creatures remembered on one combat list
  corpseDecayOffsetMs = 30000, -- milliseconds added before a generic corpse decays
  classicEquipmentSlots = false, -- hands require a weapon or a shield
  conjureFromHandsOnly = true, -- conjure reagents must be in the hands
  undergroundSeesSurface = false, -- an underground viewer cannot see the surface
  damageTextFormat = "attackerAttribution", -- name the attacker in the damage line
  -- Additive: floor(level / levelDiv) + magicLevel * c + y. c and y come from the spell.
  -- levelMult and magicMult apply only when a spell still uses scale mode.
  spell = { mode = "additive", levelDiv = 5, levelMult = 2, magicMult = 3 },
  pvpExpCap = { num = 11, den = 10 }, -- PvP kill experience treats the victim as at most level * num / den
  playerSpeed = "retail", -- "772" linear, "retail" logarithmic, "balanced" diminishing
  -- Linear catch: chance = minChance + (skill - skillBase) * skillCoeff, clamped to minChance..maxChance.
  fishing = {
    model = "linear",
    minChance = 10,
    maxChance = 50,
    skillBase = 10,
    skillCoeff = 0.597,
  },
  destroyableStone = { chance = 40, selfDamage = -50 }, -- percent chance to break the stone; damage on a miss
  otherActions = {
    changeGold = true, -- coin piles convert on use
    extraInstruments = true, -- extra instrument items have a use action
    spellbookMagicLevel = true, -- spellbook groups spells by magic level
  },
  -- Pool size in hours. Rest recovers one minute per regenSeconds. Combat spends one minute per drainSeconds. 0 hours turns it off.
  stamina = { hours = 56, regenSeconds = 180, drainSeconds = 60 },
}

-- True when the linear fishing roll succeeds for this skill.
function formulas.fishingSuccess(skill)
  local f = formulas.fishing
  local chance = math.min(
    math.max(f.minChance + (skill - f.skillBase) * f.skillCoeff, f.minChance),
    f.maxChance
  )
  return math.random(1, 100) <= chance
end
