local mType = Game.createMonsterType("Jagged Earth Elemental")
local monster = {}

monster.description = "a jagged earth elemental"
monster.experience = 1300
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
monster.health = 1500
monster.maxHealth = 1500
monster.race = "undead"
monster.speed = 280
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 20000,
	chance = 50
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

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "*STOMP STOMP*", yell = false},
}

monster.loot = {
	{id = 2148, chance = 27000, maxCount = 90}, -- gold coin
	{id = 2148, chance = 27000, maxCount = 90}, -- gold coin
	{id = 2148, chance = 1500, maxCount = 10}, -- gold coin
	{id = 2149, chance = 3750, maxCount = 2}, -- small emerald
	{id = 2245, chance = 18000}, -- twigs
	{id = 5880, chance = 800, maxCount = 2}, -- iron ore
	{id = 7732, chance = 1600}, -- seeds
	{id = 8298, chance = 9000}, -- natural soil
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
	{name = "combat", interval = 1000, chance = 10, minDamage = -100, maxDamage = -250, effect = CONST_ME_STONES, target = false, length = 6, spread = 0, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 11, minDamage = 0, maxDamage = -200, range = 7, radius = 6, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)