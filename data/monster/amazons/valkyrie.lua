local mType = Game.createMonsterType("Valkyrie")
local monster = {}

monster.description = "a valkyrie"
monster.experience = 85
monster.outfit = {
	lookType = 139,
	lookHead = 113,
	lookBody = 38,
	lookLegs = 95,
	lookFeet = 96,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 190
monster.maxHealth = 190
monster.race = "blood"
monster.speed = 176
monster.manaCost = 450
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Another head for me!", yell = false},
	{text = "Head off!", yell = false},
	{text = "Your head will be mine!", yell = false},
	{text = "Stand still!", yell = false},
	{text = "One more head for me!", yell = false},
}

monster.loot = {
	{id = 2389, chance = 55000, maxCount = 3}, -- spear
	{id = 2148, chance = 32000, maxCount = 12}, -- gold coin
	{id = 2666, chance = 30000}, -- meat
	{id = 2464, chance = 10000}, -- chain armor
	{id = 2674, chance = 7500, maxCount = 2}, -- red apple
	{id = 12399, chance = 5900}, -- girlish hair decoration
	{id = 3965, chance = 5155}, -- hunting spear
	{id = 12400, chance = 3200}, -- protective charm
	{id = 2200, chance = 1100}, -- protection amulet
	{id = 2463, chance = 830}, -- plate armor
	{id = 2229, chance = 760}, -- skull
	{id = 7618, chance = 500}, -- health potion
	{id = 2387, chance = 430}, -- double axe
	{id = 2145, chance = 130}, -- small diamond
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -50, range = 5, shootEffect = CONST_ANI_SPEAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 12,
	armor = 12,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}


mType:register(monster)