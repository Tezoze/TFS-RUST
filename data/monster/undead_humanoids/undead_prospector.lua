local mType = Game.createMonsterType("Undead Prospector")
local monster = {}

monster.description = "an undead prospector"
monster.experience = 85
monster.outfit = {
	lookType = 18,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5976
monster.health = 100
monster.maxHealth = 100
monster.race = "blood"
monster.speed = 144
monster.manaCost = 440
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
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
	{text = "Our mine... leave us alone.", yell = false},
	{text = "Turn back...", yell = false},
	{text = "These mine is ours... you shall not pass.", yell = false},
}

monster.loot = {
	{id = 2050, chance = 46150}, -- torch
	{id = 2148, chance = 53850, maxCount = 30}, -- gold coin
	{id = 2168, chance = 200}, -- life ring
	{id = 2229, chance = 240}, -- skull
	{id = 2403, chance = 15380}, -- knife
	{id = 2460, chance = 23000}, -- brass helmet
	{id = 2473, chance = 1000}, -- viking helmet
	{id = 2483, chance = 1000}, -- scale armor
	{id = 3976, chance = 92310, maxCount = 6}, -- worm
	{id = 5913, chance = 1000}, -- brown piece of cloth
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
}

monster.defenses = {
	defense = 0,
	armor = 8,
	{name = "combat", interval = 2000, chance = 15, minDamage = 5, maxDamage = 15, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)