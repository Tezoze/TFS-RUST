local mType = Game.createMonsterType("Wisp")
local monster = {}

monster.description = "a wisp"
monster.experience = 0
monster.outfit = {
	lookType = 294,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6324
monster.health = 115
monster.maxHealth = 115
monster.race = "undead"
monster.speed = 162
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = false,
	staticAttackChance = 15,
	targetDistance = 7,
	runHealth = 115,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Crackle!", yell = false},
	{text = "Tsshh", yell = false},
}

monster.loot = {
	{id = 10521, chance = 220}, -- moon backpack
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -10, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -3, maxDamage = -7, range = 1, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 10,
	armor = 7,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 200, duration = 5000},
	{name = "combat", interval = 2000, chance = 5, minDamage = 15, maxDamage = 25, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 90},
	{type = COMBAT_PHYSICALDAMAGE, percent = 65},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)