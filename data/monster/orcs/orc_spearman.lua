local mType = Game.createMonsterType("Orc Spearman")
local monster = {}

monster.description = "an orc spearman"
monster.experience = 38
monster.outfit = {
	lookType = 50,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5996
monster.health = 105
monster.maxHealth = 105
monster.race = "blood"
monster.speed = 176
monster.manaCost = 310
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
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ugaar!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 25050, maxCount = 11}, -- gold coin
	{id = 2389, chance = 17440}, -- spear
	{id = 2420, chance = 3000}, -- machete
	{id = 2468, chance = 10000}, -- studded legs
	{id = 2482, chance = 9000}, -- studded helmet
	{id = 2666, chance = 30200}, -- meat
	{id = 11113, chance = 150}, -- orc tooth
	{id = 12435, chance = 2300}, -- orc leather
	{id = 1950, chance = 1000}, -- book
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -25, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -30, range = 7, shootEffect = CONST_ANI_SPEAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 6,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)