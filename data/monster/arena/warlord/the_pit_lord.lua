local mType = Game.createMonsterType("The Pit Lord")
local monster = {}

monster.description = "The Pit Lord"
monster.experience = 2500
monster.outfit = {
	lookType = 55,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 5270
monster.maxHealth = 5270
monster.race = "blood"
monster.speed = 270
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
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "I'LL GET YOU ALL!", yell = true},
	{text = "I won't let you escape!", yell = false},
	{text = "I'll crush you beneath my feet!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -380, target = false},
	{name = "combat", interval = 7500, chance = 100, minDamage = -100, maxDamage = -250, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 44,
	armor = 46,
	{name = "speed", interval = 5000, chance = 100, effect = CONST_ME_REDSHIMMER, speed = 500, duration = 2500},
	{name = "combat", interval = 6000, chance = 65, minDamage = 20, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 1},
	{type = COMBAT_ICEDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)