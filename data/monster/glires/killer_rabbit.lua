local mType = Game.createMonsterType("Killer Rabbit")
local monster = {}

monster.description = "a killer rabbit"
monster.experience = 160
monster.outfit = {
	lookType = 74,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6017
monster.health = 205
monster.maxHealth = 205
monster.race = "blood"
monster.speed = 340
monster.manaCost = 120
monster.maxSummons = 2

monster.changeTarget = {
	interval = 5000,
	chance = 20
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Who is lunch NOW?", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 90}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 1200, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = 0, maxDamage = -50, range = 1, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 17,
	{name = "speed", interval = 1000, chance = 40, effect = CONST_ME_ENERGY, speed = 380, duration = 8000},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.summons = {
	{name = "killer rabbit", chance = 30, interval = 2000, max = 2},
}

mType:register(monster)