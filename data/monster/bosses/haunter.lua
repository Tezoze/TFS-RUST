local mType = Game.createMonsterType("Haunter")
local monster = {}

monster.description = "Haunter"
monster.experience = 4000
monster.outfit = {
	lookType = 320,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9915
monster.health = 8500
monster.maxHealth = 8500
monster.race = "blood"
monster.speed = 270
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 9
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 9,
	{text = "Surrender and I'll end it quick.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, target = false},
	{name = "combat", type = COMBAT_ENERGYDAMAGE, minDamage = 0, maxDamage = -130, interval = 2000, chance = 16, radius = 3, target = false, effect = CONST_ME_ENERGY},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 16, tick = 4000, minDamage = -20, maxDamage = -20, duration = 16000, startDamage = 13, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true},
}

monster.defenses = {
	defense = 20,
	armor = 25,
	{name = "combat", interval = 2000, chance = 16, minDamage = 100, maxDamage = 155, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 16, effect = CONST_ME_REDSHIMMER, speed = 360, duration = 80000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)