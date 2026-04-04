local mType = Game.createMonsterType("Gnorre Chyllson")
local monster = {}

monster.description = "Gnorre Chyllson"
monster.experience = 4000
monster.outfit = {
	lookType = 251,
	lookHead = 11,
	lookBody = 9,
	lookLegs = 11,
	lookFeet = 85,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 7150
monster.maxHealth = 7150
monster.race = "blood"
monster.speed = 370
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
	{text = "I am like the merciless northwind.", yell = false},
	{text = "Snow will be your death shroud.", yell = false},
	{text = "Feel the wrath of father chyll!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -455, target = false},
	{name = "melee", interval = 3000, chance = 50, minDamage = -500, maxDamage = -800, effect = CONST_ME_BLACKSPARK, target = false},
	{name = "combat", interval = 1000, chance = 15, minDamage = -170, maxDamage = -200, range = 7, shootEffect = CONST_ANI_SNOWBALL, target = true, type = COMBAT_ICEDAMAGE},
}

monster.defenses = {
	defense = 52,
	armor = 51,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -1},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)