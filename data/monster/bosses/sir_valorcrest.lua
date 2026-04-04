local mType = Game.createMonsterType("Sir Valorcrest")
local monster = {}

monster.description = "Sir Valorcrest"
monster.experience = 1800
monster.outfit = {
	lookType = 287,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8937
monster.health = 1600
monster.maxHealth = 1600
monster.race = "undead"
monster.speed = 270
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 5000,
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I challenge you!", yell = false},
	{text = "A battle makes the blood so hot and sweet.", yell = false},
}

monster.loot = {
	{id = 7427, chance = 250}, -- chaos mace
	{id = 9020, chance = 100000}, -- vampire lord token
	{id = 7588, chance = 1500}, -- strong health potion
	{id = 2207, chance = 1400}, -- sword ring
	{id = 2229, chance = 15000}, -- skull
	{id = 9020, chance = 100000}, -- vampire lord token
	{id = 2152, chance = 50000, maxCount = 5}, -- platinum coin
	{id = 2148, chance = 100000, maxCount = 93}, -- gold coin
	{id = 2534, chance = 6300}, -- vampire shield
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 70, attack = 95, target = false},
	{name = "combat", interval = 1000, chance = 12, minDamage = 0, maxDamage = -190, radius = 4, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 38,
	{name = "combat", interval = 1000, chance = 12, minDamage = 100, maxDamage = 235, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 3000, chance = 25, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -15},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Vampire", chance = 30, interval = 2000, max = 4},
}

mType:register(monster)