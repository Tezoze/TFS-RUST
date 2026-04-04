local mType = Game.createMonsterType("Deathspawn")
local monster = {}

monster.description = ""
monster.experience = 20
monster.outfit = {
	lookType = 226,
	lookHead = 114,
	lookBody = 98,
	lookLegs = 97,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 2220
monster.health = 225
monster.maxHealth = 225
monster.race = "blood"
monster.speed = 102
monster.manaCost = 305
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -40, target = false},
	{name = "combat", interval = 1000, chance = 10, minDamage = -400, maxDamage = -700, effect = CONST_ME_EXPLOSION, target = false, length = 7, spread = 0, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 11, minDamage = -250, maxDamage = -450, effect = CONST_ME_PURPLEENERGY, target = false, length = 7, spread = 0, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 5,
	armor = 1,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -15},
	{type = COMBAT_ENERGYDAMAGE, percent = -15},
	{type = COMBAT_ICEDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)