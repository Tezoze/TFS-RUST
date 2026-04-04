local mType = Game.createMonsterType("Mutated Tiger")
local monster = {}

monster.description = "a mutated tiger"
monster.experience = 750
monster.outfit = {
	lookType = 318,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9913
monster.health = 1100
monster.maxHealth = 1100
monster.race = "blood"
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
	staticAttackChance = 90,
	runHealth = 100,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "GRAAARRRRRR", yell = false},
	{text = "CHHHHHHHHHHH", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 5}, -- gold coin
	{id = 2168, chance = 5580}, -- life ring
	{id = 2515, chance = 380}, -- guardian shield
	{id = 2666, chance = 29500, maxCount = 2}, -- meat
	{id = 7436, chance = 440}, -- angelic axe
	{id = 7454, chance = 870}, -- glorious axe
	{id = 7588, chance = 6000}, -- strong health potion
	{id = 9959, chance = 730}, -- silky tapestry
	{id = 11210, chance = 20130}, -- striped fur
	{id = 11228, chance = 10600}, -- sabretooth
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -200, effect = CONST_ME_YELLOWSPARK, target = false, length = 5, spread = 3, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
	{name = "combat", interval = 2000, chance = 10, minDamage = 150, maxDamage = 300, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_EARTHDAMAGE, percent = 80},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)