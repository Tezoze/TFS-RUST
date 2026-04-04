local mType = Game.createMonsterType("Incineron")
local monster = {}

monster.description = "Incineron"
monster.experience = 3500
monster.outfit = {
	lookType = 243,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6324
monster.health = 7000
monster.maxHealth = 7000
monster.race = "fire"
monster.speed = 260
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

monster.attacks = {
	{name = "combat", interval = 2000, chance = 35, minDamage = -700, maxDamage = -1025, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 0, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 35, minDamage = 0, maxDamage = -395, range = 7, radius = 7, effect = CONST_ME_FIREAREA, target = false, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)