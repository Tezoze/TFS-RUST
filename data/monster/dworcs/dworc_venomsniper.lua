local mType = Game.createMonsterType("Dworc Venomsniper")
local monster = {}

monster.description = ""
monster.experience = 35
monster.outfit = {
	lookType = 216,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6059
monster.health = 80
monster.maxHealth = 80
monster.race = "blood"
monster.speed = 152
monster.manaCost = 300
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Brak brrretz!", yell = false},
	{text = "Grow truk grrrrr.", yell = false},
	{text = "Prek tars, dekklep zurk.", yell = false},
}

monster.loot = {
	{id = 2050, chance = 6000}, -- torch
	{id = 2148, chance = 75000, maxCount = 13}, -- gold coin
	{id = 2172, chance = 110}, -- bronze amulet
	{id = 2229, chance = 1000, maxCount = 2}, -- skull
	{id = 2411, chance = 1500}, -- poison dagger
	{id = 2467, chance = 10000}, -- leather armor
	{id = 2545, chance = 5000, maxCount = 3}, -- poison arrow
	{id = 3967, chance = 1100}, -- tribal mask
	{id = 3983, chance = 1100}, -- bast skirt
	{id = 3955, chance = 1130}, -- voodoo doll
	{id = 7732, chance = 200}, -- seeds
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -15, interval = 2000, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -20, maxDamage = -40, range = 5, target = true},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -13},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)