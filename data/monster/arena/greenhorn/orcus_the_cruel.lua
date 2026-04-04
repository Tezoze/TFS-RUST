local mType = Game.createMonsterType("Orcus the Cruel")
local monster = {}

monster.description = "Orcus the cruel"
monster.experience = 280
monster.outfit = {
	lookType = 59,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 480
monster.maxHealth = 480
monster.race = "blood"
monster.speed = 230
monster.manaCost = 0
monster.maxSummons = 0

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	targetDistance = 1,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -180, target = false},
	{name = "combat", interval = 2000, chance = 40, minDamage = 0, maxDamage = -70, range = 5, radius = 1, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 39,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)