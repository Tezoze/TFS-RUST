local mType = Game.createMonsterType("Golgordan")
local monster = {}

monster.description = "Golgordan"
monster.experience = 10000
monster.outfit = {
	lookType = 12,
	lookHead = 108,
	lookBody = 100,
	lookLegs = 105,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8721
monster.health = 40000
monster.maxHealth = 40000
monster.race = "fire"
monster.speed = 390
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 7000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Latrivan, you fool!", yell = true},
	{text = "We are the right hand and the left hand of the seven!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 273}, -- gold coin
	{id = 7591, chance = 55000}, -- great health potion
	{id = 2387, chance = 30000}, -- double axe
	{id = 6300, chance = 25000}, -- death ring
	{id = 2214, chance = 25000}, -- ring of healing
	{id = 2144, chance = 20000, maxCount = 13}, -- black pearl
	{id = 2149, chance = 20000, maxCount = 10}, -- small emerald
	{id = 2396, chance = 15000}, -- ice rapier
	{id = 2162, chance = 15000}, -- magic light wand
	{id = 2170, chance = 15000}, -- silver amulet
	{id = 2146, chance = 15000, maxCount = 10}, -- small sapphire
	{id = 2143, chance = 15000, maxCount = 13}, -- white pearl
	{id = 2520, chance = 10000}, -- demon shield
	{id = 6500, chance = 10000}, -- demonic essence
	{id = 2167, chance = 10000}, -- energy ring
	{id = 2393, chance = 10000}, -- giant sword
	{id = 9971, chance = 10000}, -- gold ingot
	{id = 2179, chance = 10000}, -- gold ring
	{id = 2470, chance = 10000}, -- golden legs
	{id = 2158, chance = 5000}, -- blue gem
	{id = 2462, chance = 5000}, -- devil helmet
	{id = 2432, chance = 5000}, -- fire axe
	{id = 2155, chance = 5000}, -- green gem
	{id = 2164, chance = 5000}, -- might ring
	{id = 2402, chance = 5000}, -- silver dagger
	{id = 2150, chance = 15000, maxCount = 12}, -- small amethyst
	{id = 2182, chance = 5000}, -- snakebite rod
	{id = 2165, chance = 5000}, -- stealth ring
	{id = 2197, chance = 5000}, -- stone skin amulet
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -500, interval = 2000, target = false},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -60, maxDamage = -200, interval = 2000, chance = 15, range = 7, radius = 4, target = true, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA},
	{name = "condition", type = CONDITION_POISON, interval = 1000, chance = 11, tick = 4000, minDamage = -30, maxDamage = -30, length = 5, spread = 0, effect = CONST_ME_POISON, target = false},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = -50, maxDamage = -600, interval = 3000, chance = 15, length = 8, spread = 3, target = false, effect = CONST_ME_MORTAREA},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = 0, maxDamage = -600, interval = 2000, chance = 10, range = 4, radius = 1, target = true, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_MORTAREA},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = 0, maxDamage = -600, interval = 2000, chance = 10, length = 8, spread = 3, target = false, effect = CONST_ME_FIREAREA},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = -50, maxDamage = -60, interval = 1000, chance = 8, radius = 6, target = false, effect = CONST_ME_GROUNDSHAKER},
}

monster.defenses = {
	defense = 54,
	armor = 48,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -1},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_PHYSICALDAMAGE, percent = 1},
	{type = COMBAT_FIREDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)