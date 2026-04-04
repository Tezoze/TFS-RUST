local mType = Game.createMonsterType("Quara Mantassin")
local monster = {}

monster.description = "a quara mantassin"
monster.experience = 400
monster.outfit = {
	lookType = 72,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6064
monster.health = 800
monster.maxHealth = 800
monster.race = "blood"
monster.speed = 590
monster.manaCost = 480
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 40,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Zuerk Pachak!", yell = false},
	{text = "Shrrrr", yell = false},
}

monster.loot = {
	{id = 2146, chance = 1000}, -- small sapphire
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 29}, -- gold coin
	{id = 2165, chance = 960}, -- stealth ring
	{id = 2377, chance = 1010}, -- two handed sword
	{id = 2381, chance = 5000}, -- halberd
	{id = 2479, chance = 100}, -- strange helmet
	{id = 2654, chance = 1050}, -- cape
	{id = 2656, chance = 50}, -- blue robe
	{id = 2670, chance = 5000, maxCount = 5}, -- shrimp
	{id = 5895, chance = 5940, maxCount = 2}, -- fish fin
	{id = 12445, chance = 11600}, -- mantassin tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -138, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 16,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 400, duration = 5000},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)