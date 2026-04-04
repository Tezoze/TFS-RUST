local mType = Game.createMonsterType("Orc Rider")
local monster = {}

monster.description = "an orc rider"
monster.experience = 110
monster.outfit = {
	lookType = 4,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6010
monster.health = 180
monster.maxHealth = 180
monster.race = "blood"
monster.speed = 260
monster.manaCost = 490
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrrrrr", yell = false},
	{text = "Orc arga Huummmak!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 980}, -- torch
	{id = 2129, chance = 10210}, -- wolf tooth chain
	{id = 2148, chance = 46000, maxCount = 81}, -- gold coin
	{id = 2425, chance = 1100}, -- obsidian lance
	{id = 2428, chance = 6880}, -- orcish axe
	{id = 2483, chance = 610}, -- scale armor
	{id = 2513, chance = 9900}, -- battle shield
	{id = 2666, chance = 24000, maxCount = 3}, -- meat
	{id = 11113, chance = 2000}, -- orc tooth
	{id = 11235, chance = 9410}, -- warwolf fur
	{id = 12435, chance = 9760}, -- orc leather
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -130, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 9,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 200, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)