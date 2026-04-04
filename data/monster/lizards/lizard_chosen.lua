local mType = Game.createMonsterType("Lizard Chosen")
local monster = {}

monster.description = "a lizard chosen"
monster.experience = 2200
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
monster.health = 3050
monster.maxHealth = 3050
monster.race = "blood"
monster.speed = 272
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grzzzzzzz!", yell = false},
	{text = "Garrrblarrrrzzzz!", yell = false},
	{text = "Kzzzzzzz!", yell = false},
}

monster.loot = {
	{id = 2145, chance = 2550, maxCount = 5}, -- small diamond
	{id = 2148, chance = 33000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 32000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 32000, maxCount = 36}, -- gold coin
	{id = 2152, chance = 2920, maxCount = 5}, -- platinum coin
	{id = 2528, chance = 1100}, -- tower shield
	{id = 5876, chance = 2000}, -- lizard leather
	{id = 5881, chance = 3980, maxCount = 3}, -- lizard scale
	{id = 7591, chance = 5350, maxCount = 3}, -- great health potion
	{id = 11301, chance = 980}, -- Zaoan armor
	{id = 11302, chance = 140}, -- Zaoan helmet
	{id = 11303, chance = 810}, -- Zaoan shoes
	{id = 11304, chance = 940}, -- Zaoan legs
	{id = 11325, chance = 9890}, -- spiked iron ball
	{id = 11326, chance = 3350}, -- corrupted flag
	{id = 11327, chance = 5800}, -- cursed shoulder spikes
	{id = 12629, chance = 2870}, -- scale of corruption
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -360, interval = 2000, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -240, maxDamage = -320, length = 3, spread = 2, effect = CONST_ME_POISON, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -190, maxDamage = -340, interval = 2000, chance = 15, radius = 3, target = false, effect = CONST_ME_GREENSPARK},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -90, maxDamage = -180, interval = 2000, chance = 10, length = 8, spread = 0, target = false, effect = CONST_ME_GREENBUBBLE},
}

monster.defenses = {
	defense = 45,
	armor = 28,
	{name = "combat", interval = 2000, chance = 10, minDamage = 75, maxDamage = 125, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
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