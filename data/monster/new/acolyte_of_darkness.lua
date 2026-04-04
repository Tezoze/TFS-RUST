local mType = Game.createMonsterType("Acolyte of Darkness")
local monster = {}

monster.description = "an acolyte of darkness"
monster.experience = 200
monster.outfit = {
	lookType = 9,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 325
monster.maxHealth = 325
monster.race = "blood"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Mine is the power of the night!", yell = false},
	{text = "You can not hope to stop us all!", yell = false},
	{text = "The power of darkness is with me!", yell = false},
}

monster.loot = {
	{id = 10531, chance = 1300}, -- midnight shard
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -120, target = false, condition = {type = CONDITION_POISON, startDamage = 160, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -60, maxDamage = -120, range = 1, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -65, maxDamage = -120, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 19,
	armor = 19,
	{name = "combat", interval = 2000, chance = 25, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = -100},
	{type = COMBAT_FIREDAMAGE, percent = -100},
	{type = COMBAT_DEATHDAMAGE, percent = -100},
	{type = COMBAT_HOLYDAMAGE, percent = 25},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)