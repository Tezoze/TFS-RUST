local mType = Game.createMonsterType("Shark")
local monster = {}

monster.description = "a shark"
monster.experience = 700
monster.outfit = {
	lookType = 453,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15287
monster.health = 1200
monster.maxHealth = 1200
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
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
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
	{text = "Rarr chomp chomp!", yell = false},
}

monster.loot = {
	{id = 2146, chance = 1222}, -- small sapphire
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 38}, -- gold coin
	{id = 2667, chance = 25000, maxCount = 4}, -- fish
	{id = 5895, chance = 161}, -- fish fin
	{id = 7632, chance = 550}, -- giant shimmering pearl
	{id = 12730, chance = 1270}, -- eye of a deepling
	{id = 12729, chance = 9090}, -- deepling scales
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -175, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 1},
	{type = COMBAT_FIREDAMAGE, percent = 1},
	{type = COMBAT_ICEDAMAGE, percent = 1},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)
