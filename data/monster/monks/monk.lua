local mType = Game.createMonsterType("Monk")
local monster = {}

monster.description = "a monk"
monster.experience = 200
monster.outfit = {
	lookType = 57,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 240
monster.maxHealth = 240
monster.race = "blood"
monster.speed = 240
monster.manaCost = 600
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
	convinceable = false,
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
	{text = "Repent Heretic!", yell = false},
	{text = "A prayer to the almighty one!", yell = false},
	{text = "I will punish the sinners!", yell = false},
}

monster.loot = {
	{id = 1949, chance = 2000}, -- scroll
	{id = 2015, chance = 820}, -- brown flask
	{id = 2044, chance = 880}, -- lamp
	{id = 2148, chance = 15000, maxCount = 18}, -- gold coin
	{id = 2166, chance = 100}, -- power ring
	{id = 2177, chance = 1002}, -- life crystal
	{id = 2193, chance = 3240}, -- ankh
	{id = 2401, chance = 440}, -- staff
	{id = 2642, chance = 710}, -- sandals
	{id = 2689, chance = 20000}, -- bread
	{id = 10563, chance = 4930}, -- book of prayers
	{id = 12448, chance = 2950}, -- rope belt
	{id = 12449, chance = 1001}, -- safety pin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -130, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 25,
	{name = "combat", interval = 2000, chance = 15, minDamage = 30, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 5000},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)