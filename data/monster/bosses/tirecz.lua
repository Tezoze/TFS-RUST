local mType = Game.createMonsterType("Tirecz")
local monster = {}

monster.description = "Tirecz"
monster.experience = 6000
monster.outfit = {
	lookType = 334,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 0
monster.health = 25000
monster.maxHealth = 25000
monster.race = "blood"
monster.speed = 220
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Hissss!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 72, attack = 100, target = false},
	{name = "combat", interval = 2000, chance = 25, range = 7, radius = 1, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 25, minDamage = -120, maxDamage = -460, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = false, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -290, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 3000, chance = 30, minDamage = -80, maxDamage = -345, effect = CONST_ME_ENERGY, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -200, maxDamage = -370, radius = 7, effect = CONST_ME_REDSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 19,
	armor = 16,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 30},
	{type = COMBAT_FIREDAMAGE, percent = 50},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)