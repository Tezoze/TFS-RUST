local mType = Game.createMonsterType("Quara Pincher")
local monster = {}

monster.description = "a quara pincher"
monster.experience = 1200
monster.outfit = {
	lookType = 77,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6063
monster.health = 1800
monster.maxHealth = 1800
monster.race = "blood"
monster.speed = 396
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Clank! Clank!", yell = false},
	{text = "Clap!", yell = false},
	{text = "Crrrk! Crrrk!", yell = false},
}

monster.loot = {
	{id = 2147, chance = 7761, maxCount = 2}, -- small ruby
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 49}, -- gold coin
	{id = 2152, chance = 40000}, -- platinum coin
	{id = 2381, chance = 6861}, -- halberd
	{id = 2475, chance = 1350}, -- warrior helmet
	{id = 2487, chance = 350}, -- crown armor
	{id = 2670, chance = 5245, maxCount = 5}, -- shrimp
	{id = 5895, chance = 1600}, -- fish fin
	{id = 7591, chance = 10630}, -- great health potion
	{id = 7897, chance = 140}, -- glacier robe
	{id = 12446, chance = 14285}, -- quara pincers
	{id = 13305, chance = 40}, -- giant shrimp
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -342, target = false},
	{name = "speed", interval = 2000, chance = 20, range = 1, effect = CONST_ME_REDSHIMMER, target = false, speed = -600, duration = 3000},
}

monster.defenses = {
	defense = 50,
	armor = 85,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -25},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)