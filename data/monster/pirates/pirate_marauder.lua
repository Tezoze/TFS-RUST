local mType = Game.createMonsterType("Pirate Marauder")
local monster = {}

monster.description = "a pirate marauder"
monster.experience = 125
monster.outfit = {
	lookType = 93,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 210
monster.maxHealth = 210
monster.race = "blood"
monster.speed = 210
monster.manaCost = 490
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 15
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 20,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Plundeeeeer!", yell = false},
	{text = "Give up!", yell = false},
	{text = "Hiyaa!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 9880}, -- torch
	{id = 2148, chance = 77670, maxCount = 40}, -- gold coin
	{id = 2389, chance = 5140, maxCount = 2}, -- spear
	{id = 2464, chance = 3000}, -- chain armor
	{id = 2510, chance = 5000}, -- plate shield
	{id = 5091, chance = 910}, -- treasure map
	{id = 5553, chance = 110}, -- rum flask
	{id = 5792, chance = 90},
	{id = 5917, chance = 880}, -- bandana
	{id = 5927, chance = 430}, -- pirate bag
	{id = 5928, chance = 80}, -- empty goldfish bowl
	{id = 6097, chance = 5200}, -- hook
	{id = 6098, chance = 5300}, -- eye patch
	{id = 6126, chance = 5200}, -- peg leg
	{id = 11219, chance = 9720}, -- compass
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -140, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -40, range = 7, shootEffect = CONST_ANI_SPEAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)