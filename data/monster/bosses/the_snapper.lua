local mType = Game.createMonsterType("The Snapper")
local monster = {}

monster.description = "The Snapper"
monster.experience = 150
monster.outfit = {
	lookType = 119,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6046
monster.health = 300
monster.maxHealth = 300
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	runHealth = 30,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 178}, -- gold coin
	{id = 7618, chance = 80000, maxCount = 5}, -- health potion
	{id = 2149, chance = 75000, maxCount = 4}, -- small emerald
	{id = 2647, chance = 44000}, -- plate legs
	{id = 2463, chance = 39800}, -- plate armor
	{id = 3982, chance = 6000}, -- crocodile boots
	{id = 2476, chance = 400}, -- knight armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 13,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 15},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)