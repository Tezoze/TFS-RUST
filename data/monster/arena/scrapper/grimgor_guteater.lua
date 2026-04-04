local mType = Game.createMonsterType("Grimgor Guteater")
local monster = {}

monster.description = "Grimgor Guteater"
monster.experience = 670
monster.outfit = {
	lookType = 2,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 1115
monster.maxHealth = 1115
monster.race = "blood"
monster.speed = 240
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

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Dont run, You're burning off precious fat.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -230, target = false},
	{name = "combat", interval = 4000, chance = 60, minDamage = -55, maxDamage = -130, range = 7, shootEffect = CONST_ANI_THROWINGSTAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 45,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -1},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)