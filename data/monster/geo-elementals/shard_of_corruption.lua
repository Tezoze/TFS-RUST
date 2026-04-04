local mType = Game.createMonsterType("Shard Of Corruption")
local monster = {}

monster.description = "a shard of corruption"
monster.experience = 5
monster.outfit = {
	lookType = 67,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6005
monster.health = 600
monster.maxHealth = 600
monster.race = "undead"
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, skill = 45, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -115, range = 7, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_GREENBUBBLE, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 60},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 25},
	{type = COMBAT_ICEDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)