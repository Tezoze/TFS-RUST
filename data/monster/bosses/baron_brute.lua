local mType = Game.createMonsterType("Baron Brute")
local monster = {}

monster.description = "Baron Brute"
monster.experience = 3000
monster.outfit = {
	lookType = 2,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6008
monster.health = 5025
monster.maxHealth = 5025
monster.race = "blood"
monster.speed = 290
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
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

monster.voices = {
	interval = 2000,
	chance = 10,
	{text = "Mash'n!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -474, target = false},
}

monster.defenses = {
	defense = 35,
	armor = 22,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 80},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)