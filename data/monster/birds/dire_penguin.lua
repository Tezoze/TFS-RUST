local mType = Game.createMonsterType("Dire Penguin")
local monster = {}

monster.description = "a dire penguin"
monster.experience = 119
monster.outfit = {
	lookType = 250,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7334
monster.health = 173
monster.maxHealth = 173
monster.race = "blood"
monster.speed = 174
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrrrr", yell = false},
}

monster.loot = {
	{id = 2148, chance = 78260, maxCount = 10}, -- gold coin
	{id = 2434, chance = 200}, -- dragon hammer
	{id = 2667, chance = 13040, maxCount = 4}, -- fish
	{id = 7158, chance = 8000}, -- rainbow trout
	{id = 7159, chance = 7000}, -- green perch
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -35, range = 7, radius = 1, shootEffect = CONST_ANI_SMALLSTONE, effect = CONST_ME_EXPLOSIONAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 2000, chance = 10, range = 7, radius = 4, effect = CONST_ME_POFF, target = false, speed = -600, duration = 9000},
}

monster.defenses = {
	defense = 16,
	armor = 16,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 310, duration = 3000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 50},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)