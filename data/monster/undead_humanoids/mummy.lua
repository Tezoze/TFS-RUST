local mType = Game.createMonsterType("Mummy")
local monster = {}

monster.description = "a mummy"
monster.experience = 150
monster.outfit = {
	lookType = 65,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6004
monster.health = 240
monster.maxHealth = 240
monster.race = "undead"
monster.speed = 150
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
	illusionable = true,
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
	{text = "I will ssswallow your sssoul!", yell = false},
	{text = "Mort ulhegh dakh visss.", yell = false},
	{text = "Flesssh to dussst!", yell = false},
	{text = "I will tassste life again!", yell = false},
	{text = "Ahkahra exura belil mort!", yell = false},
	{text = "Yohag Sssetham!", yell = false},
}

monster.loot = {
	{id = 2124, chance = 1500}, -- crystal ring
	{id = 2134, chance = 4000}, -- silver brooch
	{id = 2144, chance = 1000}, -- black pearl
	{id = 2148, chance = 38000, maxCount = 80}, -- gold coin
	{id = 2161, chance = 5000}, -- strange talisman
	{id = 2162, chance = 5800}, -- magic light wand
	{id = 2170, chance = 100}, -- silver amulet
	{id = 2411, chance = 450}, -- poison dagger
	{id = 2529, chance = 170}, -- black shield
	{id = 3976, chance = 19000, maxCount = 3}, -- worm
	{id = 5914, chance = 900}, -- yellow piece of cloth
	{id = 10566, chance = 10000}, -- gauze bandage
	{id = 11207, chance = 10}, -- mini mummy
	{id = 12422, chance = 11690}, -- flask of embalming fluid
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -85, target = false, condition = {type = CONDITION_POISON, startDamage = 4, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -30, maxDamage = -40, range = 1, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = true, speed = -226, duration = 10000},
}

monster.defenses = {
	defense = 15,
	armor = 14,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)