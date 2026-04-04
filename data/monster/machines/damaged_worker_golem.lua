local mType = Game.createMonsterType("Damaged Worker Golem")
local monster = {}

monster.description = "a damaged worker golem"
monster.experience = 95
monster.outfit = {
	lookType = 304,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9801
monster.health = 260
monster.maxHealth = 260
monster.race = "energy"
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
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
	{text = "Klonk klonk klonk", yell = false},
	{text = "Failure! Failure!", yell = false},
	{text = "Good morning citizen. How may I serve you?", yell = false},
	{text = "Target identified: Rat! Termination initiated!", yell = false},
	{text = "Rrrtttarrrttarrrtta", yell = false},
	{text = "Danger will...chrrr! Danger!", yell = false},
	{text = "Self-diagnosis failed.", yell = false},
	{text = "Aw... chhhrrr orders.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 68810, maxCount = 88}, -- gold coin
	{id = 2207, chance = 570}, -- sword ring
	{id = 5880, chance = 400}, -- iron ore
	{id = 8309, chance = 1460}, -- nail
	{id = 10572, chance = 200}, -- gear crystal
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -45, range = 7, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 16,
	{name = "combat", interval = 2000, chance = 10, minDamage = 5, maxDamage = 11, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)