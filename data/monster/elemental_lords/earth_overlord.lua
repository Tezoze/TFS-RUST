local mType = Game.createMonsterType("Earth Overlord")
local monster = {}

monster.description = "an Earth Overlord"
monster.experience = 2800
monster.outfit = {
	lookType = 285,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8933
monster.health = 4000
monster.maxHealth = 4000
monster.race = "undead"
monster.speed = 330
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 20000,
	chance = 30
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 68}, -- gold coin
	{id = 2152, chance = 33333, maxCount = 3}, -- platinum coin
	{id = 7884, chance = 1923}, -- terra mantle
	{id = 8305, chance = 100000}, -- mother soil
	{id = 11222, chance = 33333}, -- lump of earth
	{id = 11227, chance = 8333}, -- shiny stone
	{id = 8748, chance = 552},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -400, target = false},
	{name = "combat", interval = 1000, chance = 10, minDamage = 0, maxDamage = -800, effect = CONST_ME_STONES, target = false, length = 7, spread = 0, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 9, minDamage = 0, maxDamage = -490, radius = 6, effect = CONST_ME_BIGPLANTS, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 2000, chance = 20, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -750, duration = 4000},
}

monster.defenses = {
	defense = 30,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)