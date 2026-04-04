local mType = Game.createMonsterType("Ashmunrah")
local monster = {}

monster.description = "Ashmunrah"
monster.experience = 3100
monster.outfit = {
	lookType = 87,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6031
monster.health = 5000
monster.maxHealth = 5000
monster.race = "undead"
monster.speed = 430
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnFire = false,
	canWalkOnPoison = false,
	canWalkOnEnergy = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "No mortal or undead will steal my secrets!", yell = false},
	{text = "Ahhhh all those long years.", yell = false},
	{text = "My traitorous son has thee.", yell = false},
	{text = "Come to me, my allys and underlings.", yell = false},
	{text = "I might be trapped but not without power", yell = false},
	{text = "Ages come, ages go. Ashmunrah remains.", yell = false},
	{text = "You will be history soon.", yell = false},
}

monster.loot = {
	{id = 2134, chance = 7000}, -- silver brooch
	{id = 2140, chance = 400}, -- holy scarab
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 60}, -- gold coin
	{id = 2164, chance = 1000}, -- might ring
	{id = 2487, chance = 80000}, -- crown armor
	{id = 7590, chance = 1500}, -- great mana potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -1000, target = false, condition = {type = CONDITION_POISON, startDamage = 55, interval = 2000}},
	{name = "combat", interval = 3000, chance = 7, minDamage = -100, maxDamage = -700, range = 1, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 12, minDamage = -100, maxDamage = -500, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 3000, chance = 12, minDamage = -120, maxDamage = -750, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 3000, chance = 25, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -650, duration = 50000},
	{name = "combat", interval = 2000, chance = 18, minDamage = -50, maxDamage = -550, effect = CONST_ME_YELLOWBUBBLE, target = false, length = 8, spread = 3, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 30,
	armor = 25,
	{name = "combat", interval = 1000, chance = 20, minDamage = 200, maxDamage = 400, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 1000, chance = 7, effect = CONST_ME_BLUESHIMMER},
	{name = "outfit", interval = 1000, chance = 3, effect = CONST_ME_BLUESHIMMER, monster = "ancient scarab", duration = 6000},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -17},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Ancient Scarab", chance = 100, interval = 1000, max = 2},
	{name = "Green Djinn", chance = 100, interval = 1000, max = 2},
}

mType:register(monster)