local mType = Game.createMonsterType("The Dark Dancer")
local monster = {}

monster.description = "The Dark Dancer"
monster.experience = 435
monster.outfit = {
	lookType = 58,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 805
monster.maxHealth = 805
monster.race = "blood"
monster.speed = 170
monster.manaCost = 0
monster.maxSummons = 3

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	targetDistance = 1,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 5,
	{text = "I hope you like my voice!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false, condition = {type = CONDITION_POISON, startDamage = 220, interval = 2000}},
	{name = "combat", interval = 3000, chance = 70, minDamage = -60, maxDamage = -95, range = 5, radius = 1, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 6000, chance = 90, minDamage = -60, maxDamage = -95, range = 5, radius = 1, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 3500, chance = 35, range = 5, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, speed = -400, duration = 10000},
	{name = "combat", interval = 4000, chance = 30, minDamage = -2, maxDamage = -110, range = 5, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 12,
	armor = 11,
	{name = "combat", interval = 2000, chance = 45, minDamage = 75, maxDamage = 135, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 3000, chance = 50, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 40},
	{type = COMBAT_DEATHDAMAGE, percent = 1},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Ghoul", chance = 20, interval = 2000, max = 3},
}

mType:register(monster)