local mType = Game.createMonsterType("Dreadmaw")
local monster = {}

monster.description = "a dreadmaw"
monster.experience = 1500
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
monster.health = 2000
monster.maxHealth = 2000
monster.race = "blood"
monster.speed = 190
monster.manaCost = 800
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
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
	{id = 2148, chance = 99990, maxCount = 10}, -- gold coin
	{id = 9971, chance = 99990}, -- gold ingot
	{id = 2671, chance = 50000}, -- ham
	{id = 11196, chance = 99990}, -- piece of crocodile leather
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 0},
	{type = COMBAT_ICEDAMAGE, percent = 0},
	{type = COMBAT_ENERGYDAMAGE, percent = 0},
	{type = COMBAT_FIREDAMAGE, percent = 0},
	{type = COMBAT_PHYSICALDAMAGE, percent = 0},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)