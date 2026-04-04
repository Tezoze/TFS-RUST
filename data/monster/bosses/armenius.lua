local mType = Game.createMonsterType("Armenius")
local monster = {}

monster.description = "Armenius"
monster.experience = 500
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
monster.health = 550
monster.maxHealth = 550
monster.race = "undead"
monster.speed = 220
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
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "You know what, I changed my mind. BRING IT!", yell = false},
}

monster.loot = {
	{id = 2534, chance = 230}, -- vampire shield
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 50, attack = 50, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -50, maxDamage = -200, range = 1, radius = 1, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 15, radius = 1, effect = CONST_ME_REDSHIMMER, target = false, speed = -400, duration = 60000},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_GROUNDSHAKER, monster = "bat", duration = 5000},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 3000},
	{name = "combat", interval = 2000, chance = 15, minDamage = 15, maxDamage = 25, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 35},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)