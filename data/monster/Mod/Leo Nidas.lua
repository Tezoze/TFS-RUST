local mType = Game.createMonsterType("Leo Nidas")
local monster = {}

monster.description = "a Leo Nidas"
monster.experience = 0
monster.outfit = {
	lookType = 73,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 2000
monster.maxHealth = 2000
monster.race = "undead"
monster.speed = 300
monster.manaCost = 2000
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 20
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = false,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 98,
	targetDistance = 1,
	runHealth = 100,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 200, chance = 45, minDamage = -30, maxDamage = -300, skill = 153, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -190, skill = 153, target = false, type = COMBAT_HOLYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -200, radius = 3, effect = CONST_ME_EXEVO_MAS_SAN, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 30, minDamage = -50, maxDamage = -150, range = 7, shootEffect = CONST_ANI_ARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 95,
	armor = 55,
	{name = "combat", interval = 3000, chance = 35, minDamage = 250, maxDamage = 600, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 2000, chance = 15, effect = CONST_ME_EXETA_RES, type = COMBAT_NONE},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)