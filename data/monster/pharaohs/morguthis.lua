local mType = Game.createMonsterType("Morguthis")
local monster = {}

monster.description = "Morguthis"
monster.experience = 3000
monster.outfit = {
	lookType = 90,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6025
monster.health = 4800
monster.maxHealth = 4800
monster.race = "undead"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 3

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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Vengeance!", yell = false},
	{text = "You will make a fine trophy.", yell = false},
	{text = "Come and fight me, cowards!", yell = false},
	{text = "I am the supreme warrior!", yell = false},
	{text = "Let me hear the music of battle.", yell = false},
	{text = "Another one to bite the dust!", yell = false},
}

monster.loot = {
	{id = 2136, chance = 500}, -- demonbone amulet
	{id = 2144, chance = 7000}, -- black pearl
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 73}, -- gold coin
	{id = 2197, chance = 7000}, -- stone skin amulet
	{id = 2350, chance = 100000}, -- sword hilt
	{id = 2430, chance = 7000}, -- knight axe
	{id = 2443, chance = 300}, -- ravager's axe
	{id = 2645, chance = 500}, -- steel boots
	{id = 7368, chance = 500, maxCount = 3}, -- assassin star
	{id = 7591, chance = 1500}, -- great health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -1000, target = false, condition = {type = CONDITION_POISON, startDamage = 65, interval = 2000}},
	{name = "combat", interval = 3000, chance = 7, minDamage = -55, maxDamage = -550, range = 1, target = false, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 1000, chance = 25, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -650, duration = 50000},
	{name = "combat", interval = 1000, chance = 20, minDamage = -40, maxDamage = -400, radius = 3, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 3000, chance = 7, minDamage = -50, maxDamage = -500, radius = 3, effect = CONST_ME_MORTAREA, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 35,
	{name = "combat", interval = 1000, chance = 13, minDamage = 200, maxDamage = 300, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 7, effect = CONST_ME_REDSHIMMER, speed = 1201, duration = 5000},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 62},
	{type = COMBAT_FIREDAMAGE, percent = 60},
	{type = COMBAT_ENERGYDAMAGE, percent = 52},
	{type = COMBAT_EARTHDAMAGE, percent = -15},
	{type = COMBAT_HOLYDAMAGE, percent = -22},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Hero", chance = 100, interval = 2000, max = 3},
}

mType:register(monster)