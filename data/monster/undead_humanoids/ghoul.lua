local mType = Game.createMonsterType("Ghoul")
local monster = {}

monster.description = "a ghoul"
monster.experience = 85
monster.outfit = {
	lookType = 18,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5976
monster.health = 100
monster.maxHealth = 100
monster.race = "blood"
monster.speed = 144
monster.manaCost = 450
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
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2050, chance = 5000}, -- torch
	{id = 2148, chance = 68000, maxCount = 30}, -- gold coin
	{id = 2168, chance = 180}, -- life ring
	{id = 2229, chance = 240}, -- skull
	{id = 2473, chance = 990}, -- viking helmet
	{id = 2483, chance = 1000}, -- scale armor
	{id = 3976, chance = 9600, maxCount = 2}, -- worm
	{id = 5913, chance = 1000}, -- brown piece of cloth
	{id = 11208, chance = 14470}, -- rotten piece of cloth
	{id = 12423, chance = 5130}, -- ghoul snack
	{id = 12440, chance = 950}, -- pile of grave earth
	{id = 1950, chance = 1000}, -- book
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -15, maxDamage = -27, range = 1, radius = 1, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 8,
	{name = "combat", interval = 2000, chance = 5, minDamage = 9, maxDamage = 15, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)