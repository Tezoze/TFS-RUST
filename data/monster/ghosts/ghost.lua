local mType = Game.createMonsterType("Ghost")
local monster = {}

monster.description = "a ghost"
monster.experience = 120
monster.outfit = {
	lookType = 48,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5993
monster.health = 150
monster.maxHealth = 150
monster.race = "undead"
monster.speed = 160
monster.manaCost = 100
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Huh!", yell = false},
	{text = "Shhhhhh", yell = false},
	{text = "Buuuuuh", yell = false},
}

monster.loot = {
	{id = 1962, chance = 1310},
	{id = 2165, chance = 180}, -- stealth ring
	{id = 2394, chance = 10610}, -- morning star
	{id = 2404, chance = 7002}, -- combat knife
	{id = 2532, chance = 860}, -- ancient shield
	{id = 2654, chance = 8800}, -- cape
	{id = 2804, chance = 14400}, -- shadow herb
	{id = 5909, chance = 2500}, -- white piece of cloth
	{id = 10607, chance = 1870}, -- ghostly tissue
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -20, maxDamage = -45, range = 1, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.immunities = {
	{type = "physical", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)