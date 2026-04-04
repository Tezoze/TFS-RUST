local mType = Game.createMonsterType("Draptor")
local monster = {}

monster.description = "a draptor"
monster.experience = 2400
monster.outfit = {
	lookType = 382,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13316
monster.health = 3000
monster.maxHealth = 3000
monster.race = "blood"
monster.speed = 340
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 1000,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "EEHEEHEEHEEH", yell = true},
	{text = "SCREEEEECH", yell = true},
	{text = "GRRR", yell = true},
}

monster.loot = {
	{id = 2148, chance = 33750, maxCount = 90}, -- gold coin
	{id = 2148, chance = 33750, maxCount = 60}, -- gold coin
	{id = 7588, chance = 3150}, -- strong health potion
	{id = 7589, chance = 4150}, -- strong mana potion
	{id = 8867, chance = 950}, -- dragon robe
	{id = 13296, chance = 6650}, -- draptor scales
}

monster.attacks = {
	{name = "melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 3000, chance = 30, minDamage = -130, maxDamage = -310, radius = 3, effect = CONST_ME_YELLOWENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 3000, chance = 35, minDamage = -200, maxDamage = -300, range = 7, shootEffect = CONST_ANI_ENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2500, chance = 17, minDamage = -70, maxDamage = -250, length = 8, spread = 3, effect = CONST_ME_FIREAREA, target = false, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 37,
	armor = 40,
	{name = "combat", interval = 1000, chance = 25, minDamage = 57, maxDamage = 93, effect = CONST_ME_MAGIC_BLUE, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 12, effect = CONST_ME_MAGIC_RED, target = false, speed = 457, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
	{type = COMBAT_ENERGYDAMAGE, percent = 100},
	{type = COMBAT_EARTHDAMAGE, percent = -20},
	{type = COMBAT_FIREDAMAGE, percent = 50},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
