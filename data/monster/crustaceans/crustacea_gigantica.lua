local mType = Game.createMonsterType("Crustacea Gigantica")
local monster = {}

monster.description = "a crustacea gigantica"
monster.experience = 1800
monster.outfit = {
	lookType = 383,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13331
monster.health = 1600
monster.maxHealth = 1600
monster.race = "blood"
monster.speed = 240
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
	interval = 5000,
	chance = 10,
	{text = "Chrchrchr", yell = false},
	{text = "Klonklonk", yell = false},
	{text = "Chrrrrr", yell = false},
	{text = "Crunch crunch", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 43}, -- gold coin
	{id = 7589, chance = 14285}, -- strong mana potion
	{id = 13304, chance = 5000}, -- giant crab pincer
}

monster.attacks = {
	{name = "melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -600, effect = CONST_ME_DRAWBLOOD, target = false},
}

monster.defenses = {
	defense = 45,
	armor = 40,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = 100},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
