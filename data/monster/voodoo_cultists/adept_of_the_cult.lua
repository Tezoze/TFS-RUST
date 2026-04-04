local mType = Game.createMonsterType("Adept of the Cult")
local monster = {}

monster.description = "an adept of the cult"
monster.experience = 400
monster.outfit = {
	lookType = 194,
	lookHead = 114,
	lookBody = 94,
	lookLegs = 94,
	lookFeet = 57,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 430
monster.maxHealth = 430
monster.race = "blood"
monster.speed = 190
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 4000,
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
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 4,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Feel the power of the cult!", yell = false},
	{text = "Praise the voodoo!", yell = false},
	{text = "Power to the cult!", yell = false},
}

monster.loot = {
	{id = 1962, chance = 940},
	{id = 2147, chance = 320}, -- small ruby
	{id = 2148, chance = 65520, maxCount = 60}, -- gold coin
	{id = 2169, chance = 420}, -- time ring
	{id = 2170, chance = 1020}, -- silver amulet
	{id = 2183, chance = 220}, -- hailstorm rod
	{id = 2423, chance = 1260}, -- clerical mace
	{id = 2655, chance = 80}, -- red robe
	{id = 5810, chance = 1730}, -- pirate voodoo doll
	{id = 6089, chance = 2500}, -- music sheet
	{id = 7424, chance = 120}, -- lunar staff
	{id = 7426, chance = 680}, -- amber staff
	{id = 10556, chance = 10080}, -- cultish robe
	{id = 12411, chance = 90}, -- cultish symbol
	{id = 12448, chance = 10000}, -- rope belt
	{id = 12608, chance = 120}, -- broken key ring
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false, condition = {type = CONDITION_POISON, startDamage = 2, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -70, maxDamage = -150, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, range = 7, radius = 1, shootEffect = CONST_ANI_HOLY, effect = CONST_ME_HOLYDAMAGE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 33,
	{name = "combat", interval = 3000, chance = 20, minDamage = 45, maxDamage = 60, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_YELLOWBUBBLE},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 40},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Ghoul", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)