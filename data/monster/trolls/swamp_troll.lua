local mType = Game.createMonsterType("Swamp Troll")
local monster = {}

monster.description = "a swamp troll"
monster.experience = 25
monster.outfit = {
	lookType = 76,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6018
monster.health = 55
monster.maxHealth = 55
monster.race = "venom"
monster.speed = 128
monster.manaCost = 320
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrr", yell = false},
	{text = "Groar!", yell = false},
	{text = "Me strong! Me ate spinach!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 15000}, -- torch
	{id = 2148, chance = 50300, maxCount = 5}, -- gold coin
	{id = 2235, chance = 10000}, -- mouldy cheese
	{id = 2389, chance = 13000}, -- spear
	{id = 2580, chance = 60}, -- fishing rod
	{id = 2643, chance = 9500}, -- leather boots
	{id = 2667, chance = 60000}, -- fish
	{id = 2805, chance = 1200}, -- troll green
	{id = 5901, chance = 2140}, -- wood
	{id = 10603, chance = 3100}, -- swamp grass
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -13, target = false, condition = {type = CONDITION_POISON, startDamage = 1, interval = 2000}},
}

monster.defenses = {
	defense = 15,
	armor = 6,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = -5},
}


mType:register(monster)