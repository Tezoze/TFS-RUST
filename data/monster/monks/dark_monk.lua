local mType = Game.createMonsterType("Dark Monk")
local monster = {}

monster.description = "a dark monk"
monster.experience = 145
monster.outfit = {
	lookType = 225,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 190
monster.maxHealth = 190
monster.race = "blood"
monster.speed = 230
monster.manaCost = 480
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You are no match to us!", yell = false},
	{text = "This is where your path will end!", yell = false},
	{text = "Your end has come!", yell = false},
}

monster.loot = {
	{id = 1949, chance = 1790}, -- scroll
	{id = 2015, chance = 380}, -- brown flask
	{id = 2044, chance = 550}, -- lamp
	{id = 2148, chance = 14600, maxCount = 18}, -- gold coin
	{id = 2166, chance = 120}, -- power ring
	{id = 2177, chance = 990}, -- life crystal
	{id = 2193, chance = 3900}, -- ankh
	{id = 2642, chance = 890}, -- sandals
	{id = 2689, chance = 20550}, -- bread
	{id = 7620, chance = 790}, -- mana potion
	{id = 10563, chance = 1900}, -- book of prayers
	{id = 11220, chance = 10500}, -- dark rosary
	{id = 12448, chance = 6666}, -- rope belt
	{id = 12449, chance = 990}, -- safety pin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -20, maxDamage = -50, range = 1, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 22,
	{name = "combat", interval = 2000, chance = 15, minDamage = 25, maxDamage = 49, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 6000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 40},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)