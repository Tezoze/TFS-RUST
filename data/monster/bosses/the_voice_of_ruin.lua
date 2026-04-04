local mType = Game.createMonsterType("The Voice of Ruin")
local monster = {}

monster.description = "a the voice of ruin"
monster.experience = 3500
monster.outfit = {
	lookType = 344,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11288
monster.health = 5500
monster.maxHealth = 5500
monster.race = "blood"
monster.speed = 460
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 40
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 80,
	runHealth = 50,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 33000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 32000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 32000, maxCount = 36}, -- gold coin
	{id = 11326, chance = 5800}, -- corrupted flag
	{id = 2152, chance = 2920, maxCount = 5}, -- platinum coin
	{id = 11327, chance = 3800}, -- cursed shoulder spikes
	{id = 9971, chance = 3800}, -- gold ingot
	{id = 11325, chance = 3800}, -- spiked iron ball
	{id = 11303, chance = 3800}, -- zaoan shoes
}

monster.attacks = {
	{name = "melee", interval = 2000, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -440, maxDamage = -820, length = 3, spread = 2, effect = CONST_ME_POISON, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -290, maxDamage = -540, interval = 2000, chance = 15, radius = 3, target = false, effect = CONST_ME_GREENSPARK},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -190, maxDamage = -480, interval = 2000, chance = 10, length = 8, spread = 0, target = false, effect = CONST_ME_GREENBUBBLE},
}

monster.defenses = {
	defense = 45,
	armor = 45,
	{name = "combat", interval = 2000, chance = 10, minDamage = 475, maxDamage = 625, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)