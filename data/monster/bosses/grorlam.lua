local mType = Game.createMonsterType("Grorlam")
local monster = {}

monster.description = "Grorlam"
monster.experience = 2400
monster.outfit = {
	lookType = 205,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6005
monster.health = 3000
monster.maxHealth = 3000
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 3
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

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 20}, -- gold coin
	{id = 2483, chance = 10000}, -- scale armor
	{id = 1294, chance = 20000, maxCount = 5}, -- small stone
	{id = 2395, chance = 2500}, -- carlin sword
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 75, attack = 60, target = false},
	{name = "combat", interval = 1000, chance = 15, minDamage = -150, maxDamage = -200, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 15,
	{name = "combat", interval = 1000, chance = 25, minDamage = 100, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 6, effect = CONST_ME_REDSHIMMER, speed = 270, duration = 6000},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 100},
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)