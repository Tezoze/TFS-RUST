local mType = Game.createMonsterType("Orc Marauder")
local monster = {}

monster.description = "an orc marauder"
monster.experience = 205
monster.outfit = {
	lookType = 342,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11251
monster.health = 235
monster.maxHealth = 235
monster.race = "blood"
monster.speed = 390
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
	canPushCreatures = true,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrrrr", yell = false},
}

monster.loot = {
	{id = 2148, chance = 55000, maxCount = 90}, -- gold coin
	{id = 2425, chance = 1110}, -- obsidian lance
	{id = 2428, chance = 1320}, -- orcish axe
	{id = 2455, chance = 440}, -- crossbow
	{id = 2456, chance = 5210}, -- bow
	{id = 2666, chance = 24600}, -- meat
	{id = 8857, chance = 70}, -- silkweaver bow
	{id = 11113, chance = 3890}, -- orc tooth
	{id = 11324, chance = 10090}, -- shaggy tail
	{id = 12407, chance = 4830}, -- broken crossbow
	{id = 12435, chance = 3800}, -- orc leather
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 50, minDamage = 0, maxDamage = -100, range = 7, shootEffect = CONST_ANI_ONYXARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 16,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 350, duration = 5000},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 5},
	{type = COMBAT_EARTHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)