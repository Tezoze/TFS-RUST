local mType = Game.createMonsterType("The Plasmother")
local monster = {}

monster.description = "The Plasmother"
monster.experience = 12000
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
monster.health = 7500
monster.maxHealth = 7500
monster.race = "venom"
monster.speed = 310
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 5500,
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
	runHealth = 250,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Blubb", yell = false},
	{text = "Blubb Blubb", yell = false},
	{text = "Blubberdiblubb", yell = false},
}

monster.loot = {
	{id = 2148, chance = 20000, maxCount = 177}, -- gold coin
	{id = 2152, chance = 25000, maxCount = 13}, -- platinum coin
	{id = 6500, chance = 45000}, -- demonic essence
	{id = 2144, chance = 5000, maxCount = 3}, -- black pearl
	{id = 2146, chance = 5000, maxCount = 3}, -- small sapphire
	{id = 5944, chance = 35000}, -- soul orb
	{id = 6535, chance = 100000}, -- the plasmother's remains
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 30, attack = 50, target = false},
	{name = "speed", interval = 1000, chance = 8, radius = 6, effect = CONST_ME_POISON, target = false, speed = -800, duration = 10000},
	{name = "combat", interval = 2000, chance = 15, minDamage = -200, maxDamage = -350, radius = 4, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 3000, chance = 15, minDamage = -200, maxDamage = -530, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENSPARK, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 1000, chance = 75, minDamage = 505, maxDamage = 605, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -20},
	{type = COMBAT_ENERGYDAMAGE, percent = 15},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Defiler", chance = 20, interval = 4000, max = 2},
}

mType:register(monster)