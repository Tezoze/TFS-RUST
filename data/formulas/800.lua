-- Adjustable 8.0 mechanics. Spell damage uses its own additive formula. The stamina pool is on.

formulas = {
  defenseGateMs = 2000, -- minimum time between defense rolls
  armor = "randomized", -- "randomized" rolls half the armor, "full" subtracts it
  distanceKeep = "perType", -- "perType" uses each monster's targetDistance; a number forces one distance
  damageFormula = "classic", -- "classic" is the probe roll, "modern" is the level-and-skill formula
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
  expAttributionRounds = 60, -- combat rounds that still count toward kill experience
  combatListSlots = 20, -- creatures remembered on one combat list
  corpseDecayOffsetMs = 30000, -- milliseconds added before a generic corpse decays
  classicEquipmentSlots = true, -- hands accept any pickupable item
  conjureFromHandsOnly = true, -- conjure reagents must be in the hands
  undergroundSeesSurface = true, -- an underground viewer can see the surface
  damageTextFormat = "attackerAttribution", -- name the attacker in the damage line
  -- Multipliers for offensive and defensive stance. Balanced stance stays at 1.
  fightModes = {
    offensiveAtk = 1.20, defensiveAtk = 0.60,
    offensiveDef = 0.60, defensiveDef = 1.80,
  },
  -- Additive: floor(level / levelDiv) + magicLevel * c + y. c and y come from the spell.
  -- levelMult and magicMult apply only when a spell still uses scale mode.
  spell = { mode = "additive", levelDiv = 5, levelMult = 2, magicMult = 3 },
  pvpExpCap = { num = 11, den = 10 }, -- PvP kill experience treats the victim as at most level * num / den
  playerSpeed = "classic", -- "classic" linear, "retail" logarithmic, "balanced" diminishing
  -- Percent lost from experience and skills. Promoted characters use promoted. Each blessing subtracts perBlessing.
  deathLossPercent = { base = 10, promoted = 7, perBlessing = 1 },
  -- Probe catch: skill must beat a roll below diff, then a 0..99 roll must fall within prob.
  fishing = { model = "probe", diff = 80, prob = 50 },
  destroyableStone = { chance = 40, selfDamage = -50 }, -- percent chance to break the stone; damage on a miss
  -- Hit points and mana restored by an equipped regen item each tick.
  creatures = {
    itemRegenHp = 1,
    itemRegenMana = 4,
  },
  raidWaveMaxCount = 64, -- monsters spawned in one raid wave
  -- Condition length. cycle is the tick, count and max are how many ticks run.
  skillTimers = {
    haste = { cycle = 3, count = 10, max = 10 },
    strongHaste = { cycle = 2, count = 10, max = 10 },
    paralyze = { cycle = 1, count = 10, max = 10 },
    manaShield = { cycle = 1, count = 200, max = 200 },
    invisible = { cycle = 1, count = 200, max = 200 },
    light = { [6] = 500, [8] = 1000, [9] = 2000 }, -- duration in ms for light radius 6, 8, and 9
  },
  otherActions = {
    changeGold = false, -- coin piles convert on use
    extraInstruments = false, -- extra instrument items have a use action
    spellbookMagicLevel = false, -- spellbook groups spells by magic level
  },
  -- Pool size in hours. Rest recovers one minute per regenSeconds. Combat spends one minute per drainSeconds. 0 hours turns it off.
  stamina = { hours = 56, regenSeconds = 180, drainSeconds = 60 },
}

-- True when the probe fishing roll succeeds for this skill.
function formulas.fishingSuccess(skill)
  local f = formulas.fishing
  if f.diff == 0 then
    return math.random(0, 99) <= f.prob
  end
  return skill >= math.random(0, f.diff - 1) and math.random(0, 99) <= f.prob
end
