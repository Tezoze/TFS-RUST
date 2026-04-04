local mType = Game.createMonsterType("Sharptooth")
local monster = {}

monster.description = "Sharptooth"
monster.experience = 1600
monster.outfit = {
	lookType = 20,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6067
monster.health = 3100
monster.maxHealth = 3100
monster.race = "blood"
monster.speed = 390
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Gnarrr!", yell = false},
	{text = "Tcharrr!", yell = false},
	{text = "Rrrah!", yell = false},
	{text = "Rraaar!", yell = false},
}

monster.loot = {
	{id = 2226, chance = 50000}, -- fishbone
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -500, target = false},
}

monster.defenses = {
	defense = 29,
	armor = 20,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_GREENSHIMMER, speed = 310},
	{name = "combat", interval = 2000, chance = 12, minDamage = 200, maxDamage = 240, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_EARTHDAMAGE, percent = 80},
}

monster.immunities = {
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)