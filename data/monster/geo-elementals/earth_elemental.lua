local mType = Game.createMonsterType("Earth Elemental")
local monster = {}

monster.description = "an earth elemental"
monster.experience = 450
monster.outfit = {
	lookType = 301,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8933
monster.health = 650
monster.maxHealth = 650
monster.race = "undead"
monster.speed = 230
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 80,
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Stomp.", yell = false},
}

monster.loot = {
	{id = 1294, chance = 10000, maxCount = 10}, -- small stone
	{id = 2148, chance = 43000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 30}, -- gold coin
	{id = 2260, chance = 10000}, -- blank rune
	{id = 7589, chance = 1910}, -- strong mana potion
	{id = 7850, chance = 20160, maxCount = 30}, -- earth arrow
	{id = 9808, chance = 350},
	{id = 9970, chance = 620}, -- small topaz
	{id = 11222, chance = 20460}, -- lump of earth
	{id = 11339, chance = 570}, -- clay lump
	{id = 8748, chance = 470},
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -110, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -72, maxDamage = -105, interval = 2000, chance = 10, range = 7, target = true, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_GREENBUBBLE},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = 0, maxDamage = -100, interval = 2000, chance = 10, range = 7, radius = 2, target = true, shootEffect = CONST_ANI_LARGEROCK, effect = CONST_ME_POFF},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 20, tick = 4000, minDamage = -200, maxDamage = -260, length = 6, spread = 0, effect = CONST_ME_BIGPLANTS, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -100, maxDamage = -140, radius = 5, effect = CONST_ME_POISONAREA, target = false},
	{name = "speed", interval = 2000, chance = 10, range = 5, target = true, effect = CONST_ME_SMALLPLANTS, speed = -330, duration = 5000},
}

monster.defenses = {
	defense = 25,
	armor = 45,
	{name = "combat", interval = 2000, chance = 5, minDamage = 60, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = 85},
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
}


mType:register(monster)