local mType = Game.createMonsterType("Smuggler")
local monster = {}

monster.description = "a smuggler"
monster.experience = 48
monster.outfit = {
	lookType = 134,
	lookHead = 95,
	lookBody = 0,
	lookLegs = 113,
	lookFeet = 115,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 130
monster.maxHealth = 130
monster.race = "blood"
monster.speed = 176
monster.manaCost = 390
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "I will silence you forever!", yell = false},
	{text = "You saw something you shouldn't!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 30200, maxCount = 2}, -- torch
	{id = 2148, chance = 80000, maxCount = 10}, -- gold coin
	{id = 2376, chance = 5000}, -- sword
	{id = 2403, chance = 9920}, -- knife
	{id = 2404, chance = 4400}, -- combat knife
	{id = 2406, chance = 10000}, -- short sword
	{id = 2461, chance = 10050}, -- leather helmet
	{id = 2649, chance = 14840}, -- leather legs
	{id = 2671, chance = 10200}, -- ham
	{id = 7397, chance = 110}, -- deer trophy
	{id = 8840, chance = 5000, maxCount = 5}, -- raspberry
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)