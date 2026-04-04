local mType = Game.createMonsterType("Clay Guardian")
local monster = {}

monster.description = "a clay guardian"
monster.experience = 400
monster.outfit = {
	lookType = 333,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13972
monster.health = 625
monster.maxHealth = 625
monster.race = "undead"
monster.speed = 210
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
	staticAttackChance = 60,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 1294, chance = 10000, maxCount = 10}, -- small stone
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 63}, -- gold coin
	{id = 2260, chance = 25000}, -- blank rune
	{id = 7850, chance = 5555, maxCount = 8}, -- earth arrow
	{id = 9970, chance = 320}, -- small topaz
	{id = 11222, chance = 25000}, -- lump of earth
	{id = 11339, chance = 1100}, -- clay lump
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -125, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -30, maxDamage = -150, range = 7, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_GREENBUBBLE, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 50, minDamage = -20, maxDamage = -30, radius = 3, effect = CONST_ME_POFF, target = false, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 42,
	{name = "combat", interval = 2000, chance = 10, minDamage = 40, maxDamage = 130, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 40},
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)