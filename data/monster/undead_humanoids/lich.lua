local mType = Game.createMonsterType("Lich")
local monster = {}

monster.description = "a lich"
monster.experience = 900
monster.outfit = {
	lookType = 99,
	lookHead = 95,
	lookBody = 116,
	lookLegs = 119,
	lookFeet = 115,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6028
monster.health = 880
monster.maxHealth = 880
monster.race = "undead"
monster.speed = 210
monster.manaCost = 0
monster.maxSummons = 4

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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Doomed be the living!", yell = false},
	{text = "Death awaits all!", yell = false},
	{text = "Thy living flesh offends me!", yell = false},
	{text = "Death and Decay!", yell = false},
	{text = "You will endure agony beyond thy death!", yell = false},
	{text = "Pain sweet pain!", yell = false},
	{text = "Come to me my children!", yell = false},
}

monster.loot = {
	{id = 2143, chance = 5000}, -- white pearl
	{id = 2144, chance = 5960, maxCount = 3}, -- black pearl
	{id = 2148, chance = 100000, maxCount = 139}, -- gold coin
	{id = 2149, chance = 2230, maxCount = 3}, -- small emerald
	{id = 2152, chance = 19720}, -- platinum coin
	{id = 2154, chance = 690}, -- yellow gem
	{id = 2171, chance = 450}, -- platinum amulet
	{id = 2175, chance = 10000}, -- spellbook
	{id = 2178, chance = 350}, -- mind stone
	{id = 2214, chance = 1540}, -- ring of healing
	{id = 2436, chance = 550}, -- skull staff
	{id = 2479, chance = 740}, -- strange helmet
	{id = 2532, chance = 2422}, -- ancient shield
	{id = 2535, chance = 350}, -- castle shield
	{id = 2656, chance = 150}, -- blue robe
	{id = 7589, chance = 7500}, -- strong mana potion
	{id = 7893, chance = 200}, -- lightning boots
	{id = 9970, chance = 2430, maxCount = 3}, -- small topaz
	{id = 13291, chance = 150}, -- Maxilla Maximus
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -75, interval = 2000, target = false},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -140, maxDamage = -190, interval = 2000, chance = 10, length = 7, spread = 0, target = false, effect = CONST_ME_REDSHIMMER},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -300, maxDamage = -400, length = 7, spread = 0, effect = CONST_ME_HITBYPOISON, target = false},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -200, maxDamage = -245, interval = 2000, chance = 10, range = 1, target = true, effect = CONST_ME_REDSHIMMER},
	{name = "speed", interval = 2000, chance = 15, range = 7, target = true, effect = CONST_ME_REDSHIMMER, speed = -300, duration = 30000},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -130, maxDamage = -195, interval = 2000, chance = 10, radius = 3, target = false, effect = CONST_ME_REDSHIMMER},
}

monster.defenses = {
	defense = 25,
	armor = 50,
	{name = "combat", interval = 2000, chance = 15, minDamage = 80, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 80},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Bonebeast", chance = 10, interval = 2000, max = 4},
}

mType:register(monster)