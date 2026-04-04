local mType = Game.createMonsterType("Dworc Fleshhunter")
local monster = {}

monster.description = "a dworc fleshhunter"
monster.experience = 40
monster.outfit = {
	lookType = 215,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6058
monster.health = 85
monster.maxHealth = 85
monster.race = "blood"
monster.speed = 148
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 8,
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
	{id = 2050, chance = 4750}, -- torch
	{id = 2148, chance = 69000, maxCount = 13}, -- gold coin
	{id = 2229, chance = 3300, maxCount = 3}, -- skull
	{id = 2411, chance = 2250}, -- poison dagger
	{id = 2541, chance = 1000}, -- bone shield
	{id = 2568, chance = 9750}, -- cleaver
	{id = 3964, chance = 90}, -- ripper lance
	{id = 3965, chance = 2000}, -- hunting spear
	{id = 3967, chance = 1000}, -- tribal mask
	{id = 3955, chance = 1130}, -- voodoo doll
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -25, target = false, condition = {type = CONDITION_POISON, startDamage = 20, interval = 2000}},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -15, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 3,
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = -15},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)