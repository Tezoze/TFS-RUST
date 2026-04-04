local mType = Game.createMonsterType("Doom Deer")
local monster = {}

monster.description = "a doom deer"
monster.experience = 200
monster.outfit = {
	lookType = 31,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5970
monster.health = 405
monster.maxHealth = 405
monster.race = "blood"
monster.speed = 182
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 20
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
	runHealth = 25,
	canWalkOnFire = false,
	canWalkOnPoison = false,
	canWalkOnEnergy = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "I bet it was you who killed my mom!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 4000, chance = 30, minDamage = -35, maxDamage = -55, effect = CONST_ME_BIGCLOUDS, target = false, length = 5, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 19,
	{name = "speed", interval = 3000, chance = 30, effect = CONST_ME_REDSHIMMER, speed = 400, duration = 8000},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)