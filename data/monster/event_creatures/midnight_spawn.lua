local mType = Game.createMonsterType("Midnight Spawn")
local monster = {}

monster.description = "a midnight spawn"
monster.experience = 900
monster.outfit = {
	lookType = 315,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9960
monster.health = 2000
monster.maxHealth = 2000
monster.race = "undead"
monster.speed = 340
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 10531, chance = 8333}, -- midnight shard
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 40,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = 99},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 99},
	{type = COMBAT_LIFEDRAINDAMAGE, percent = 99},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)