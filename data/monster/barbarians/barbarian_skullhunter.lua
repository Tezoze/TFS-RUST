local mType = Game.createMonsterType("Barbarian Skullhunter")
local monster = {}

monster.description = "a barbarian skullhunter"
monster.experience = 85
monster.outfit = {
	lookType = 254,
	lookHead = 0,
	lookBody = 77,
	lookLegs = 77,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 135
monster.maxHealth = 135
monster.race = "blood"
monster.speed = 168
monster.manaCost = 450
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 70,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You will become my trophy.", yell = false},
	{text = "Fight harder, coward.", yell = false},
	{text = "Show that you are a worthy opponent.", yell = false},
}

monster.loot = {
	{id = 2050, chance = 6680}, -- torch
	{id = 2148, chance = 8240, maxCount = 30}, -- gold coin
	{id = 2168, chance = 300}, -- life ring
	{id = 2229, chance = 3000}, -- skull
	{id = 2403, chance = 1067}, -- knife
	{id = 2460, chance = 2200}, -- brass helmet
	{id = 2473, chance = 860}, -- viking helmet
	{id = 2483, chance = 440}, -- scale armor
	{id = 5913, chance = 500}, -- brown piece of cloth
	{id = 7449, chance = 100}, -- crystal sword
	{id = 7457, chance = 100}, -- fur boots
	{id = 7462, chance = 100}, -- ragnir helmet
	{id = 7618, chance = 100}, -- health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
}

monster.defenses = {
	defense = 0,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)