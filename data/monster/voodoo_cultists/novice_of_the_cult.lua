local mType = Game.createMonsterType("Novice of the Cult")
local monster = {}

monster.description = "a novice of the cult"
monster.experience = 100
monster.outfit = {
	lookType = 133,
	lookHead = 114,
	lookBody = 95,
	lookLegs = 114,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 285
monster.maxHealth = 285
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 1

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
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 40,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Fear us!", yell = false},
	{text = "You will not tell anyone what you have seen!", yell = false},
	{text = "Your curiosity will be punished!", yell = false},
}

monster.loot = {
	{id = 1962, chance = 700},
	{id = 2145, chance = 210}, -- small diamond
	{id = 2148, chance = 43380, maxCount = 40}, -- gold coin
	{id = 2190, chance = 450}, -- wand of vortex
	{id = 2199, chance = 420}, -- garlic necklace
	{id = 2213, chance = 500}, -- dwarven ring
	{id = 2661, chance = 2900}, -- scarf
	{id = 5810, chance = 520}, -- pirate voodoo doll
	{id = 6087, chance = 2500}, -- music sheet
	{id = 10556, chance = 1030}, -- cultish robe
	{id = 12448, chance = 5910}, -- rope belt
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -65, target = false, condition = {type = CONDITION_POISON, startDamage = 1, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -20, maxDamage = -80, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 15, minDamage = 20, maxDamage = 40, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -8},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -8},
}

monster.summons = {
	{name = "Chicken", chance = 10, interval = 2000, max = 1},
}

mType:register(monster)