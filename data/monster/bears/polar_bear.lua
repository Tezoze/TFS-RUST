local mType = Game.createMonsterType("Polar Bear")
local monster = {}

monster.description = "a polar bear"
monster.experience = 28
monster.outfit = {
	lookType = 42,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5987
monster.health = 85
monster.maxHealth = 85
monster.race = "blood"
monster.speed = 156
monster.manaCost = 315
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
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 5,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "GROARRR!", yell = true},
}

monster.loot = {
	{id = 2666, chance = 50500, maxCount = 4}, -- meat
	{id = 2671, chance = 50320, maxCount = 2}, -- ham
	{id = 10567, chance = 980}, -- polar bear paw
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 7,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 1},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}


mType:register(monster)