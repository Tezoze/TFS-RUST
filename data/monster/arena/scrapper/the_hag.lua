local mType = Game.createMonsterType("The Hag")
local monster = {}

monster.description = "The Hag"
monster.experience = 510
monster.outfit = {
	lookType = 264,
	lookHead = 78,
	lookBody = 97,
	lookLegs = 95,
	lookFeet = 95,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 935
monster.maxHealth = 935
monster.race = "blood"
monster.speed = 205
monster.manaCost = 0
monster.maxSummons = 2

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	targetDistance = 5,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "If you think I am to old to fight then you're wrong!", yell = false},
	{text = "I've forgotten more things then you have ever learned!", yell = false},
	{text = "Let me teach you a few things youngster!", yell = false},
	{text = "I'll teach you respect for the old!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 3000, chance = 35, range = 5, radius = 1, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 4000, chance = 55, range = 5, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, speed = -400, duration = 12000},
}

monster.defenses = {
	defense = 25,
	armor = 24,
	{name = "combat", interval = 2000, chance = 35, minDamage = 95, maxDamage = 155, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 3000, chance = 50, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Ghost", chance = 26, interval = 2000, max = 2},
	{name = "Crypt Shambler", chance = 26, interval = 2000, max = 2},
}

mType:register(monster)