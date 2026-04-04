local mType = Game.createMonsterType("The Evil Eye")
local monster = {}

monster.description = "the Evil Eye"
monster.experience = 750
monster.outfit = {
	lookType = 210,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6037
monster.health = 1200
monster.maxHealth = 1200
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 5

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
	canPushCreatures = false,
	targetDistance = 3,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Inferior creatures, bow before my power!", yell = false},
	{text = "653768764!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 45}, -- gold coin
	{id = 5898, chance = 5000}, -- bonelord eye
	{id = 2148, chance = 80000, maxCount = 90}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 65, attack = 24, target = false},
	{name = "combat", interval = 1000, chance = 15, minDamage = -60, maxDamage = -130, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 13, minDamage = -85, maxDamage = -115, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 17, minDamage = -135, maxDamage = -175, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 15, minDamage = -40, maxDamage = -120, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 12, minDamage = -110, maxDamage = -130, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 1000, chance = 10, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -850, duration = 20000},
	{name = "combat", interval = 1000, chance = 8, minDamage = -35, maxDamage = -85, effect = CONST_ME_GREENBUBBLE, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 6, minDamage = -75, maxDamage = -85, effect = CONST_ME_REDSHIMMER, target = false, length = 8, spread = 3, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 1000, chance = 9, minDamage = -150, maxDamage = -250, effect = CONST_ME_BLUEBUBBLE, target = false, length = 8, spread = 3, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 23,
	armor = 19,
	{name = "combat", interval = 1000, chance = 9, minDamage = 1, maxDamage = 219, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "demon skeleton", chance = 13, interval = 1000, max = 5},
	{name = "ghost", chance = 12, interval = 1000, max = 4},
}

mType:register(monster)