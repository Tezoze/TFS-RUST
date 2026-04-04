local mType = Game.createMonsterType("Enlightened of the Cult")
local monster = {}

monster.description = "an enlightened of the cult"
monster.experience = 500
monster.outfit = {
	lookType = 193,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 700
monster.maxHealth = 700
monster.race = "blood"
monster.speed = 200
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
	canPushCreatures = true,
	staticAttackChance = 50,
	targetDistance = 4,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Praise to my master Urgith!", yell = false},
	{text = "You will rise as my servant!", yell = false},
	{text = "Praise to my masters! Long live the triangle!", yell = false},
	{text = "You will die in the name of the triangle!", yell = false},
}

monster.loot = {
	{id = 1962, chance = 910},
	{id = 2114, chance = 130}, -- piggy bank
	{id = 2146, chance = 550}, -- small sapphire
	{id = 2148, chance = 64550, maxCount = 70}, -- gold coin
	{id = 2167, chance = 450}, -- energy ring
	{id = 2171, chance = 200}, -- platinum amulet
	{id = 2187, chance = 180}, -- wand of inferno
	{id = 2200, chance = 790}, -- protection amulet
	{id = 2436, chance = 350}, -- skull staff
	{id = 2656, chance = 40}, -- blue robe
	{id = 5801, chance = 100}, -- jewelled backpack
	{id = 5810, chance = 430}, -- pirate voodoo doll
	{id = 6090, chance = 2500}, -- music sheet
	{id = 7426, chance = 100}, -- amber staff
	{id = 7589, chance = 740}, -- strong mana potion
	{id = 10555, chance = 10250}, -- cultish mask
	{id = 12411, chance = 890}, -- cultish symbol
	{id = 12608, chance = 100}, -- broken key ring
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false, condition = {type = CONDITION_POISON, startDamage = 4, interval = 2000}},
	{name = "combat", interval = 2000, chance = 25, minDamage = -70, maxDamage = -185, range = 1, radius = 1, shootEffect = CONST_ANI_HOLY, effect = CONST_ME_HOLYAREA, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_HOLY, effect = CONST_ME_HOLYDAMAGE, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 2000, chance = 10, range = 7, effect = CONST_ME_REDSHIMMER, target = true, speed = -360, duration = 6000},
}

monster.defenses = {
	defense = 25,
	armor = 40,
	{name = "combat", interval = 2000, chance = 25, minDamage = 60, maxDamage = 90, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_YELLOWBUBBLE},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 5},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Crypt Shambler", chance = 10, interval = 2000, max = 2},
	{name = "Ghost", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)