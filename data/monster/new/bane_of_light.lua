local mType = Game.createMonsterType("Bane of Light")
local monster = {}

monster.description = "a bane of light"
monster.experience = 750
monster.outfit = {
	lookType = 68,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6006
monster.health = 1100
monster.maxHealth = 1100
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 30,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 10531, chance = 6930}, -- midnight shard
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -366, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -50, maxDamage = -200, range = 1, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 15, range = 1, effect = CONST_ME_REDSHIMMER, target = true, speed = -400, duration = 60000},
}

monster.defenses = {
	defense = 38,
	armor = 38,
	{name = "outfit", interval = 4000, chance = 10, effect = CONST_ME_GROUNDSHAKER, monster = "bat", duration = 5000},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 3000},
	{name = "combat", interval = 2000, chance = 15, minDamage = 15, maxDamage = 25, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -100},
	{type = COMBAT_ICEDAMAGE, percent = -100},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)