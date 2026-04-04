local mType = Game.createMonsterType("Lizard Snakecharmer")
local monster = {}

monster.description = "a lizard snakecharmer"
monster.experience = 210
monster.outfit = {
	lookType = 115,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6041
monster.health = 325
monster.maxHealth = 325
monster.race = "blood"
monster.speed = 184
monster.manaCost = 0
monster.maxSummons = 6

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
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 80,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "I smeeeel warm blood!", yell = false},
	{text = "Shhhhhhh", yell = false},
}

monster.loot = {
	{id = 2148, chance = 83740, maxCount = 55}, -- gold coin
	{id = 2150, chance = 520}, -- small amethyst
	{id = 2154, chance = 150}, -- yellow gem
	{id = 2168, chance = 340}, -- life ring
	{id = 2177, chance = 1430}, -- life crystal
	{id = 2181, chance = 920}, -- terra rod
	{id = 2182, chance = 230}, -- snakebite rod
	{id = 2654, chance = 8640}, -- cape
	{id = 3971, chance = 230}, -- charmer's tiara
	{id = 5876, chance = 1320}, -- lizard leather
	{id = 5881, chance = 3860}, -- lizard scale
	{id = 7620, chance = 860}, -- mana potion
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -30, interval = 2000, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -100, maxDamage = -200, range = 7, shootEffect = CONST_ANI_POISON, target = true},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -50, maxDamage = -110, interval = 2000, chance = 15, range = 7, radius = 1, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENBUBBLE},
}

monster.defenses = {
	defense = 15,
	armor = 22,
	{name = "combat", interval = 2000, chance = 50, minDamage = 50, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}

monster.summons = {
	{name = "cobra", chance = 20, interval = 2000, max = 6},
}

mType:register(monster)