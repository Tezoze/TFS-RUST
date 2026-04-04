local mType = Game.createMonsterType("Orc Sambackpack")
local monster = {}

monster.name = "Orc"
monster.description = "an orc"
monster.experience = 25
monster.outfit = {
	lookType = 5,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5966
monster.health = 70
monster.maxHealth = 70
monster.race = "blood"
monster.speed = 150
monster.manaCost = 300
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
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
	{text = "Grak brrretz!", yell = false},
	{text = "Grow truk grrrrr.", yell = false},
	{text = "Prek tars, dekklep zurk.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 84810, maxCount = 14}, -- gold coin
	{id = 3960, chance = 100000}, -- old and used backpack
	{id = 2385, chance = 5850}, -- sabre
	{id = 2386, chance = 4960}, -- axe
	{id = 2482, chance = 2950}, -- studded helmet
	{id = 2484, chance = 7860}, -- studded armor
	{id = 2526, chance = 7300}, -- studded shield
	{id = 2666, chance = 10160}, -- meat
	{id = 11113, chance = 210}, -- orc tooth
	{id = 12435, chance = 590}, -- orc leather
	{id = 1950, chance = 1000}, -- book
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)