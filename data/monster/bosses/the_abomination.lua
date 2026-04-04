local mType = Game.createMonsterType("The Abomination")
local monster = {}

monster.description = "the Abomination"
monster.experience = 25000
monster.outfit = {
	lookType = 238,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6532
monster.health = 38050
monster.maxHealth = 38050
monster.race = "venom"
monster.speed = 340
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
	illusionable = true,
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
	{text = "Blubb", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 10000, maxCount = 3}, -- platinum coin
	{id = 6500, chance = 2857}, -- demonic essence
	{id = 5944, chance = 2500}, -- soul orb
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 90, attack = 120, target = false},
	{name = "speed", interval = 1000, chance = 12, radius = 6, effect = CONST_ME_POISON, target = false, speed = -800, duration = 10000},
	{name = "combat", interval = 1000, chance = 9, minDamage = -200, maxDamage = -650, radius = 4, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 11, minDamage = -400, maxDamage = -900, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENNOTE, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 19, minDamage = -350, maxDamage = -850, shootEffect = CONST_ANI_POISON, target = false, length = 7, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 1000, chance = 75, minDamage = 505, maxDamage = 605, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "poison", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)