local mType = Game.createMonsterType("Rift Scythe")
local monster = {}

monster.description = "a rift scythe"
monster.experience = 2000
monster.outfit = {
	lookType = 300,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 0
monster.health = 3600
monster.maxHealth = 3600
monster.race = "undead"
monster.speed = 370
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
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
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -380, target = false},
	{name = "combat", interval = 2000, chance = 60, minDamage = 0, maxDamage = -200, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 50, minDamage = 0, maxDamage = -600, effect = CONST_ME_REDSPARK, target = false, length = 7, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 40, minDamage = 0, maxDamage = -395, radius = 4, effect = CONST_ME_REDSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 3000, chance = 60, minDamage = 0, maxDamage = -300, effect = CONST_ME_EXPLOSIONAREA, target = false, length = 7, spread = 3, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 1000, chance = 25, minDamage = 100, maxDamage = 195, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -20},
	{type = COMBAT_DEATHDAMAGE, percent = 60},
	{type = COMBAT_FIREDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)