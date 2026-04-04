local mType = Game.createMonsterType("Terror Bird")
local monster = {}

monster.description = "a terror bird"
monster.experience = 150
monster.outfit = {
	lookType = 218,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6057
monster.health = 300
monster.maxHealth = 300
monster.race = "blood"
monster.speed = 212
monster.manaCost = 490
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
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
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "CRAAAHHH!", yell = false},
	{text = "Gruuuh Gruuuh.", yell = false},
	{text = "Carrah! Carrah!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 82430, maxCount = 30}, -- gold coin
	{id = 2666, chance = 48550, maxCount = 3}, -- meat
	{id = 11190, chance = 10310}, -- terrorbird beak
	{id = 3976, chance = 9540, maxCount = 3}, -- worm
	{id = 12470, chance = 3090}, -- colourful feather
	{id = 7618, chance = 660}, -- health potion
	{id = 7732, chance = 240}, -- seeds
	{id = 3970, chance = 100}, -- feather headdress
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false},
}

monster.defenses = {
	defense = 13,
	armor = 13,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)