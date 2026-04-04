local mType = Game.createMonsterType("The Collector")
local monster = {}

monster.description = "the Collector"
monster.experience = 100
monster.outfit = {
	lookType = 261,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 10612
monster.health = 340
monster.maxHealth = 340
monster.race = "undead"
monster.speed = 195
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 5
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 50,
	targetDistance = 1,
	runHealth = 20,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Leave as long as you can.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 100, attack = 40, target = false},
	{name = "speed", interval = 1000, chance = 13, effect = CONST_ME_ENERGY, target = false, length = 8, spread = 0, speed = -800, duration = 20000},
	{name = "combat", interval = 1000, chance = 15, minDamage = 0, maxDamage = -85, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "melee", interval = 2000, chance = 15, minDamage = -10, maxDamage = -80, range = 7, radius = 3, effect = CONST_ME_BLACKSPARK, target = false},
}

monster.defenses = {
	defense = 26,
	armor = 25,
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)