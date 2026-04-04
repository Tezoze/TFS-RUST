local mType = Game.createMonsterType("Gargoyle")
local monster = {}

monster.description = "a gargoyle"
monster.experience = 150
monster.outfit = {
	lookType = 95,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6027
monster.health = 250
monster.maxHealth = 250
monster.race = "undead"
monster.speed = 200
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 30,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Harrrr harrrr!", yell = false},
	{text = "Stone sweet stone.", yell = false},
	{text = "Feel my claws, softskin.", yell = false},
	{text = "Chhhhhrrrrk!", yell = false},
	{text = "There is a stone in your shoe!", yell = false},
}

monster.loot = {
	{id = 2129, chance = 1480}, -- wolf tooth chain
	{id = 2148, chance = 88000, maxCount = 30}, -- gold coin
	{id = 2207, chance = 260}, -- melee ring
	{id = 2394, chance = 2150}, -- morning star
	{id = 2457, chance = 850}, -- steel helmet
	{id = 2489, chance = 300}, -- dark armor
	{id = 2513, chance = 1000}, -- battle shield
	{id = 2680, chance = 1810, maxCount = 5}, -- strawberry
	{id = 8838, chance = 9220, maxCount = 2}, -- potato
	{id = 11195, chance = 11730}, -- stone wing
	{id = 11227, chance = 190}, -- shiny stone
	{id = 11343, chance = 630}, -- piece of marble rock
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -65, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 26,
	{name = "combat", interval = 2000, chance = 20, minDamage = 5, maxDamage = 15, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 40},
	{type = COMBAT_DEATHDAMAGE, percent = 1},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)