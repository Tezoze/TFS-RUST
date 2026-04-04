local mType = Game.createMonsterType("Orc")
local monster = {}

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
	{id = 2148, chance = 84760, maxCount = 14}, -- gold coin
	{id = 2666, chance = 9630}, -- meat
	{id = 2484, chance = 8180}, -- studded armor
	{id = 2526, chance = 6750}, -- studded shield
	{id = 2385, chance = 6110}, -- sabre
	{id = 2386, chance = 5330}, -- axe
	{id = 2482, chance = 3060}, -- studded helmet
	{id = 1950, chance = 2020}, -- book
	{id = 12435, chance = 460}, -- orc leather
	{id = 11113, chance = 50}, -- orc tooth
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