local mType = Game.createMonsterType("Bear")
local monster = {}

monster.description = ""
monster.experience = 23
monster.outfit = {
	lookType = 16,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5975
monster.health = 80
monster.maxHealth = 80
monster.race = "blood"
monster.speed = 156
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
	pushable = false,
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
	{text = "Grrrr", yell = false},
	{text = "Groar", yell = false},
}

monster.loot = {
	{id = 2666, chance = 39750, maxCount = 4}, -- meat
	{id = 2671, chance = 20000, maxCount = 3}, -- ham
	{id = 5896, chance = 5000}, -- bear paw
	{id = 5902, chance = 2460}, -- honeycomb
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -25, target = false},
}

monster.defenses = {
	defense = 6,
	armor = 6,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)