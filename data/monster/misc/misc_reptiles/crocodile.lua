local mType = Game.createMonsterType("Crocodile")
local monster = {}

monster.description = "a crocodile"
monster.experience = 40
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
monster.health = 105
monster.maxHealth = 105
monster.race = "blood"
monster.speed = 120
monster.manaCost = 350
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
	convinceable = true,
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
	{id = 2148, chance = 50000, maxCount = 10}, -- gold coin
	{id = 2671, chance = 40000}, -- ham
	{id = 3982, chance = 100}, -- crocodile boots
	{id = 11196, chance = 20180}, -- piece of crocodile leather
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -40, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 13,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}


mType:register(monster)