local mType = Game.createMonsterType("Mercury Blob")
local monster = {}

monster.description = "a mercury blob"
monster.experience = 180
monster.outfit = {
	lookType = 316,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9961
monster.health = 150
monster.maxHealth = 150
monster.race = "undead"
monster.speed = 150
monster.manaCost = 0
monster.maxSummons = 3

monster.changeTarget = {
	interval = 5000,
	chance = 0
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
	targetDistance = 1,
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Crackle", yell = false},
}

monster.loot = {
	{id = 9966, chance = 18750}, -- glob of mercury
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -75, target = false},
	{name = "combat", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_HOLY, effect = CONST_ME_STUN, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -10, maxDamage = -30, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 1,
	armor = 3,
	{name = "combat", interval = 2000, chance = 5, minDamage = 20, maxDamage = 30, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 65},
	{type = COMBAT_ICEDAMAGE, percent = 15},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 65},
	{type = COMBAT_LIFEDRAINDAMAGE, percent = 80},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
}

monster.summons = {
	{name = "Mercury Blob", chance = 10, interval = 2000, max = 3},
}

mType:register(monster)