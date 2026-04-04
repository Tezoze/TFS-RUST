local mType = Game.createMonsterType("Norgle Glacierbeard")
local monster = {}

monster.description = "Norgle Glacierbeard"
monster.experience = 2100
monster.outfit = {
	lookType = 257,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 4280
monster.maxHealth = 4280
monster.race = "blood"
monster.speed = 195
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
	{text = "I'll extinguish you warmbloods.", yell = false},
	{text = "REVENGE!", yell = false},
	{text = "Far too hot.", yell = false},
	{text = "DISGUSTING WARMBLOODS!", yell = false},
	{text = "Revenge is sweetest when served cold.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -225, target = false},
}

monster.defenses = {
	defense = 27,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = -1},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)