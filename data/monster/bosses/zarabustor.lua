local mType = Game.createMonsterType("Zarabustor")
local monster = {}

monster.description = "zarabustor"
monster.experience = 8000
monster.outfit = {
	lookType = 130,
	lookHead = 0,
	lookBody = 77,
	lookLegs = 92,
	lookFeet = 115,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 5100
monster.maxHealth = 5100
monster.race = "blood"
monster.speed = 220
monster.manaCost = 0
monster.maxSummons = 3

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
	canPushCreatures = true,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 900,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Killing is such a splendid diversion from my studies.", yell = false},
	{text = "Time to test my newest spells!", yell = false},
	{text = "Ah, practice time once again!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 32000, maxCount = 80}, -- gold coin
	{id = 2411, chance = 9600}, -- poison dagger
	{id = 2436, chance = 8330}, -- skull staff
	{id = 7368, chance = 5500, maxCount = 4}, -- assassin star
	{id = 2656, chance = 3390}, -- blue robe
	{id = 2146, chance = 3190}, -- small sapphire
	{id = 7898, chance = 3040}, -- lightning robe
	{id = 2123, chance = 2420}, -- ring of the sky
	{id = 2466, chance = 2240}, -- golden armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -130, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -250, range = 7, radius = 3, shootEffect = CONST_ANI_BURSTARROW, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 2000, chance = 10, range = 7, radius = 2, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 2000, chance = 25, minDamage = 0, maxDamage = -250, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -130, maxDamage = -350, effect = CONST_ME_BIGCLOUDS, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -250, range = 7, target = false, type = COMBAT_MANADRAIN},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -330, duration = 20000},
	{name = "combat", interval = 2000, chance = 5, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 20,
	{name = "combat", interval = 2000, chance = 20, minDamage = 100, maxDamage = 225, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 95},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Warlock", chance = 10, interval = 2000, max = 2},
	{name = "Green Djinn", chance = 10, interval = 2000, max = 3},
}

mType:register(monster)