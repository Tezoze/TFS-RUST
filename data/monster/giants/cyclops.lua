local mType = Game.createMonsterType("Cyclops")
local monster = {}

monster.description = "a cyclops"
monster.experience = 150
monster.outfit = {
	lookType = 22,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5962
monster.health = 260
monster.maxHealth = 260
monster.race = "blood"
monster.speed = 190
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
	{text = "Il lorstok human!", yell = false},
	{text = "Toks utat.", yell = false},
	{text = "Human, uh whil dyh!", yell = false},
	{text = "Youh ah trak!", yell = false},
	{text = "Let da mashing begin!", yell = false},
}

monster.loot = {
	{id = 2129, chance = 190}, -- wolf tooth chain
	{id = 2148, chance = 82000, maxCount = 47}, -- gold coin
	{id = 2207, chance = 90}, -- melee ring
	{id = 2381, chance = 1003}, -- halberd
	{id = 2406, chance = 8000}, -- short sword
	{id = 2490, chance = 220}, -- dark helmet
	{id = 2510, chance = 2500}, -- plate shield
	{id = 2513, chance = 1400}, -- battle shield
	{id = 2666, chance = 30070}, -- meat
	{id = 7398, chance = 80}, -- cyclops trophy
	{id = 7618, chance = 210}, -- health potion
	{id = 10574, chance = 4930}, -- cyclops toe
	{id = 1950, chance = 1000}, -- book
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -105, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 17,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 25},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)