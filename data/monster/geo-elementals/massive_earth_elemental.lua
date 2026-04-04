local mType = Game.createMonsterType("Massive Earth Elemental")
local monster = {}

monster.description = "a massive earth elemental"
monster.experience = 950
monster.outfit = {
	lookType = 285,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8933
monster.health = 1330
monster.maxHealth = 1330
monster.race = "undead"
monster.speed = 370
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
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
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 1294, chance = 25280, maxCount = 10}, -- small stone
	{id = 2148, chance = 32000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 32000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 32000, maxCount = 41}, -- gold coin
	{id = 2197, chance = 980}, -- stone skin amulet
	{id = 2200, chance = 1580}, -- protection amulet
	{id = 2213, chance = 2790}, -- dwarven ring
	{id = 7387, chance = 150}, -- diamond sceptre
	{id = 7887, chance = 500}, -- terra amulet
	{id = 9809, chance = 3300},
	{id = 9970, chance = 5280, maxCount = 2}, -- small topaz
	{id = 11222, chance = 40680}, -- lump of earth
	{id = 11339, chance = 480}, -- clay lump
	{id = 8748, chance = 430},
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -110, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -99, maxDamage = -145, interval = 2000, chance = 10, range = 7, target = true, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_GREENBUBBLE},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = -95, maxDamage = -169, interval = 2000, chance = 10, range = 7, radius = 2, target = true, shootEffect = CONST_ANI_LARGEROCK, effect = CONST_ME_POFF},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -300, maxDamage = -320, length = 6, spread = 0, effect = CONST_ME_BIGPLANTS, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -200, maxDamage = -220, radius = 5, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true},
	{name = "speed", interval = 2000, chance = 20, range = 5, target = true, effect = CONST_ME_SMALLPLANTS, speed = -330, duration = 5000},
}

monster.defenses = {
	defense = 35,
	armor = 60,
	{name = "combat", interval = 2000, chance = 5, minDamage = 150, maxDamage = 180, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 45},
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
}


mType:register(monster)