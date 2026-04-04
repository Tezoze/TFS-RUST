local mType = Game.createMonsterType("Frost Giant")
local monster = {}

monster.description = "a frost giant"
monster.experience = 150
monster.outfit = {
	lookType = 257,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7330
monster.health = 270
monster.maxHealth = 270
monster.race = "blood"
monster.speed = 190
monster.manaCost = 490
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hmm Humansoup!", yell = false},
	{text = "Stand still ya tasy snack!", yell = false},
	{text = "Joh Thun!", yell = false},
	{text = "Bröre Smöde!", yell = false},
	{text = "Hörre Sjan Flan!", yell = false},
	{text = "Forle Bramma", yell = false},
}

monster.loot = {
	{id = 2148, chance = 82000, maxCount = 40}, -- gold coin
	{id = 2207, chance = 130}, -- melee ring
	{id = 2381, chance = 560}, -- halberd
	{id = 2406, chance = 8140}, -- short sword
	{id = 2490, chance = 180}, -- dark helmet
	{id = 2513, chance = 1350}, -- battle shield
	{id = 2666, chance = 4970, maxCount = 2}, -- meat
	{id = 7290, chance = 600}, -- shard
	{id = 7441, chance = 2180}, -- ice cube
	{id = 7460, chance = 250}, -- norse shield
	{id = 7618, chance = 819}, -- health potion
	{id = 10575, chance = 5000}, -- frost giant pelt
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -90, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 22,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 5000},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)