local mType = Game.createMonsterType("Duskbringer")
local monster = {}

monster.description = "a duskbringer"
monster.experience = 2600
monster.outfit = {
	lookType = 300,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8955
monster.health = 3550
monster.maxHealth = 3550
monster.race = "undead"
monster.speed = 370
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 20
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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Death!", yell = false},
	{text = "Come a little closer!", yell = false},
	{text = "The end is near!", yell = false},
}

monster.loot = {
	{id = 10531, chance = 10000}, -- midnight shard
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -350, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -165, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -350, maxDamage = -720, effect = CONST_ME_REDSPARK, target = false, length = 8, spread = 0, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -300, effect = CONST_ME_EXPLOSIONAREA, target = false, length = 7, spread = 3, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -225, maxDamage = -275, radius = 4, effect = CONST_ME_REDSPARK, target = false, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 42,
	armor = 42,
	{name = "combat", interval = 2000, chance = 15, minDamage = 130, maxDamage = 205, effect = CONST_ME_REDSPARK, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 450, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = -80},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -30},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = -40},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)