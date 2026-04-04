local mType = Game.createMonsterType("Battlemaster Zunzu")
local monster = {}

monster.description = "Battlemaster Zunzu"
monster.experience = 2500
monster.outfit = {
	lookType = 343,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11281
monster.health = 4000
monster.maxHealth = 4000
monster.race = "blood"
monster.speed = 420
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
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 150,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Hissss!", yell = false},
}

monster.loot = {
	{id = 7591, chance = 2775, maxCount = 2}, -- great health potion
	{id = 11206, chance = 100000}, -- red lantern
	{id = 11301, chance = 1050}, -- Zaoan armor
	{id = 11303, chance = 3150}, -- Zaoan shoes
	{id = 11304, chance = 2625}, -- Zaoan legs
	{id = 11330, chance = 11250}, -- zaogun flag
	{id = 11331, chance = 100000}, -- zaogun shoulderplates
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = -115, maxDamage = -350, range = 1, radius = 1, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 45,
	{name = "combat", interval = 1000, chance = 18, minDamage = 200, maxDamage = 400, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 25},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 15},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)