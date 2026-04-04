local mType = Game.createMonsterType("Ice Overlord")
local monster = {}

monster.description = "an Ice Overlord"
monster.experience = 2800
monster.outfit = {
	lookType = 11,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8965
monster.health = 4000
monster.maxHealth = 4000
monster.race = "undead"
monster.speed = 390
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 20000,
	chance = 15
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 85,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 38}, -- gold coin
	{id = 2152, chance = 50000, maxCount = 3}, -- platinum coin
	{id = 8300, chance = 100000}, -- flawless ice crystal
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -400, target = false},
	{name = "speed", interval = 2000, chance = 18, radius = 6, effect = CONST_ME_ICETORNADO, target = false, speed = -800, duration = 5000},
	{name = "combat", interval = 1000, chance = 9, minDamage = -50, maxDamage = -400, range = 7, shootEffect = CONST_ANI_SMALLICE, effect = CONST_ME_ICEATTACK, target = true, type = COMBAT_ICEDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 15, minDamage = 90, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)