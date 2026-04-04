local mType = Game.createMonsterType("The Handmaiden")
local monster = {}

monster.description = "the Handmaiden"
monster.experience = 7500
monster.outfit = {
	lookType = 230,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6312
monster.health = 19500
monster.maxHealth = 19500
monster.race = "blood"
monster.speed = 450
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 3100,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 6539, chance = 35000}, -- the handmaiden's protector
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -800, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = -150, maxDamage = -800, range = 7, effect = CONST_ME_BLUESHIMMER, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 1000, chance = 12, range = 1, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 25,
	{name = "speed", interval = 3000, chance = 12, effect = CONST_ME_REDSHIMMER, speed = 380, duration = 8000},
	{name = "invisible", interval = 4000, chance = 50, effect = CONST_ME_REDSHIMMER},
	{name = "combat", interval = 2000, chance = 50, minDamage = 100, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 35, effect = CONST_ME_REDSHIMMER, speed = 370, duration = 30000},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "poison", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)