local mType = Game.createMonsterType("Xenia")
local monster = {}

monster.description = "Xenia"
monster.experience = 255
monster.outfit = {
	lookType = 137,
	lookHead = 95,
	lookBody = 115,
	lookLegs = 115,
	lookFeet = 95,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 200
monster.maxHealth = 200
monster.race = "blood"
monster.speed = 176
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2229, chance = 100000, maxCount = 2}, -- skull
	{id = 2148, chance = 66666, maxCount = 34}, -- gold coin
	{id = 2385, chance = 33000}, -- sabre
	{id = 2526, chance = 33000}, -- studded shield
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
	{name = "combat", interval = 2000, chance = 10, effect = CONST_ME_REDNOTE, target = false, length = 3, spread = 2, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 320, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 7},
	{type = COMBAT_DEATHDAMAGE, percent = -7},
	{type = COMBAT_PHYSICALDAMAGE, percent = -6},
}


mType:register(monster)