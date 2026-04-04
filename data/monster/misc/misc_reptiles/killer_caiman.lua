local mType = Game.createMonsterType("Killer Caiman")
local monster = {}

monster.description = "a killer caiman"
monster.experience = 900
monster.outfit = {
	lookType = 358,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11430
monster.health = 1500
monster.maxHealth = 1500
monster.race = "blood"
monster.speed = 230
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2149, chance = 10150, maxCount = 5}, -- small emerald
	{id = 2425, chance = 4975}, -- obsidian lance
	{id = 2671, chance = 40100}, -- ham
	{id = 3982, chance = 510}, -- crocodile boots
	{id = 7632, chance = 1130},
	{id = 11196, chance = 25430}, -- piece of crocodile leather
	{id = 11245, chance = 4800, maxCount = 2}, -- bunch of ripe rice
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -180, target = false},
}

monster.defenses = {
	defense = 35,
	armor = 40,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 700, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)