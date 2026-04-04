local mType = Game.createMonsterType("Midnight Panther")
local monster = {}

monster.description = "a midnight panther"
monster.experience = 900
monster.outfit = {
	lookType = 385,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13327
monster.health = 1200
monster.maxHealth = 1200
monster.race = "blood"
monster.speed = 290
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
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Groooooooar", yell = false},
	{text = "MEOW", yell = true},
	{text = "Groarrrrrrrr", yell = false},
	{text = "Purrrrrrr", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 57}, -- gold coin
	{id = 3052, chance = 12500}, -- life ring
	{id = 2666, chance = 25000, maxCount = 4}, -- meat
	{id = 13026, chance = 12500}, -- panther head
	{id = 13027, chance = 100000}, -- panther paw
}

monster.attacks = {
	{name = "melee", interval = 1500, chance = 100, minDamage = 0, maxDamage = -90, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -75, maxDamage = -215, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGYAREA, target = false, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_MAGIC_RED, target = false, speed = 370, duration = 5000},
	{name = "combat", interval = 2000, chance = 15, minDamage = 50, maxDamage = 125, effect = CONST_ME_MAGIC_BLUE, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_MAGIC_BLUE},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 100},
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
