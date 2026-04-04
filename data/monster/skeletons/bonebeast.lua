local mType = Game.createMonsterType("Bonebeast")
local monster = {}

monster.description = "a bonebeast"
monster.experience = 580
monster.outfit = {
	lookType = 101,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6030
monster.health = 515
monster.maxHealth = 515
monster.race = "undead"
monster.speed = 218
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Cccchhhhhhhhh!", yell = false},
	{text = "Knooorrrrr!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 14000, maxCount = 50}, -- gold coin
	{id = 2148, chance = 16000, maxCount = 40}, -- gold coin
	{id = 2229, chance = 20000}, -- skull
	{id = 2230, chance = 47750}, -- bone
	{id = 2449, chance = 4950}, -- bone club
	{id = 2463, chance = 8000}, -- plate armor
	{id = 2541, chance = 2000}, -- bone shield
	{id = 2796, chance = 1350}, -- green mushroom
	{id = 5925, chance = 3960}, -- hardened bone
	{id = 7618, chance = 540}, -- health potion
	{id = 11161, chance = 120}, -- bonebeast trophy
	{id = 11194, chance = 9780}, -- bony tail
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -200, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -50, maxDamage = -90, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -25, maxDamage = -47, interval = 2000, chance = 10, radius = 3, target = false, effect = CONST_ME_REDSHIMMER},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -50, maxDamage = -60, radius = 3, effect = CONST_ME_POISON, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -70, maxDamage = -80, length = 6, spread = 0, effect = CONST_ME_POISON, target = false},
	{name = "speed", interval = 2000, chance = 15, range = 7, target = true, speed = -600, duration = 13000},
}

monster.defenses = {
	defense = 30,
	armor = 40,
	{name = "combat", interval = 2000, chance = 15, minDamage = 50, maxDamage = 60, effect = CONST_ME_GREENSPARK, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)