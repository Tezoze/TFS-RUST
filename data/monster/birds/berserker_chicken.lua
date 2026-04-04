local mType = Game.createMonsterType("Berserker Chicken")
local monster = {}

monster.description = ""
monster.experience = 220
monster.outfit = {
	lookType = 111,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6042
monster.health = 465
monster.maxHealth = 465
monster.race = "blood"
monster.speed = 166
monster.manaCost = 0
monster.maxSummons = 0

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
	{text = "Gokgoooook", yell = false},
	{text = "Cluck Cluck", yell = false},
	{text = "I will fill MY cushion with YOUR hair! CLUCK!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 1200, minDamage = 0, maxDamage = -200, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = 0, maxDamage = -100, range = 1, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -41, maxDamage = -70, effect = CONST_ME_EXPLOSIONAREA, target = false, type = COMBAT_DROWNDAMAGE},
}

monster.defenses = {
	defense = 12,
	armor = 12,
	{name = "speed", interval = 1000, chance = 40, effect = CONST_ME_REDSHIMMER, speed = 400, duration = 8000},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)