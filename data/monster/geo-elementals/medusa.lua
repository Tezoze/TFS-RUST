local mType = Game.createMonsterType("Medusa")
local monster = {}

monster.description = "a medusa"
monster.experience = 4050
monster.outfit = {
	lookType = 330,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 10524
monster.health = 4500
monster.maxHealth = 4500
monster.race = "blood"
monster.speed = 250
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 20
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
	staticAttackChance = 80,
	runHealth = 600,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You will make sssuch a fine ssstatue!", yell = false},
	{text = "There isss no chhhanccce of essscape", yell = false},
	{text = "Jussst look at me!", yell = false},
	{text = "Are you tired or why are you moving thhat ssslow <chuckle>", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 90}, -- gold coin
	{id = 2149, chance = 3770, maxCount = 4}, -- small emerald
	{id = 2152, chance = 74810, maxCount = 6}, -- platinum coin
	{id = 2476, chance = 1840}, -- knight armor
	{id = 2536, chance = 3040}, -- medusa shield
	{id = 7413, chance = 1160}, -- titan axe
	{id = 7590, chance = 10000, maxCount = 2}, -- great mana potion
	{id = 7884, chance = 870}, -- terra mantle
	{id = 7885, chance = 420}, -- terra legs
	{id = 7887, chance = 4060}, -- terra amulet
	{id = 8473, chance = 9290, maxCount = 2}, -- ultimate health potion
	{id = 9810, chance = 500},
	{id = 10219, chance = 850}, -- sacred tree amulet
	{id = 11226, chance = 9900}, -- strand of medusa hair
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -450, target = false, condition = {type = CONDITION_POISON, startDamage = 840, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -21, maxDamage = -350, range = 7, shootEffect = CONST_ANI_EARTH, effect = CONST_ME_CARNIPHILA, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = -250, maxDamage = -500, effect = CONST_ME_CARNIPHILA, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 2000, chance = 25, radius = 7, effect = CONST_ME_POFF, target = true, speed = -900},
	{name = "outfit", interval = 2000, chance = 1, range = 7, target = true},
}

monster.defenses = {
	defense = 30,
	armor = 45,
	{name = "combat", interval = 2000, chance = 25, minDamage = 150, maxDamage = 300, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)