local mType = Game.createMonsterType("Stone Golem")
local monster = {}

monster.description = "a stone golem"
monster.experience = 160
monster.outfit = {
	lookType = 67,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6005
monster.health = 270
monster.maxHealth = 270
monster.race = "undead"
monster.speed = 180
monster.manaCost = 590
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 1294, chance = 13890, maxCount = 4}, -- small stone
	{id = 2124, chance = 120}, -- crystal ring
	{id = 2148, chance = 90000, maxCount = 40}, -- gold coin
	{id = 2156, chance = 30}, -- red gem
	{id = 2166, chance = 5070}, -- power ring
	{id = 2395, chance = 2500}, -- carlin sword
	{id = 5880, chance = 1980}, -- iron ore
	{id = 10549, chance = 1020}, -- ancient stone
	{id = 11227, chance = 760}, -- shiny stone
	{id = 11232, chance = 10370}, -- sulphurous stone
	{id = 11343, chance = 380}, -- piece of marble rock
	{id = 8748, chance = 550},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 15},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)