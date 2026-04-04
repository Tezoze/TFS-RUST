local mType = Game.createMonsterType("Barbarian Headsplitter")
local monster = {}

monster.description = "a barbarian headsplitter"
monster.experience = 85
monster.outfit = {
	lookType = 253,
	lookHead = 115,
	lookBody = 105,
	lookLegs = 119,
	lookFeet = 132,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 100
monster.maxHealth = 100
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
	{text = "I will regain my honor with your blood!", yell = false},
	{text = "Surrender is not option!", yell = false},
	{text = "Its you or me!", yell = false},
	{text = "Die! Die! Die!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 60300}, -- torch
	{id = 2148, chance = 75600, maxCount = 30}, -- gold coin
	{id = 2168, chance = 230}, -- life ring
	{id = 2403, chance = 14890}, -- knife
	{id = 2460, chance = 20140}, -- brass helmet
	{id = 2473, chance = 5020}, -- viking helmet
	{id = 2229, chance = 8000, maxCount = 2}, -- skull
	{id = 2483, chance = 4060}, -- scale armor
	{id = 5913, chance = 980}, -- brown piece of cloth
	{id = 7457, chance = 90}, -- fur boots
	{id = 7461, chance = 110}, -- krimhorn helmet
	{id = 7618, chance = 560}, -- health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -60, range = 7, radius = 1, shootEffect = CONST_ANI_WHIRLWINDAXE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 7,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)