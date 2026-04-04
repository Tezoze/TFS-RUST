local mType = Game.createMonsterType("Thieving Squirrel")
local monster = {}

monster.description = "a thieving squirrel"
monster.experience = 15
monster.outfit = {
	lookType = 274,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7628
monster.health = 55
monster.maxHealth = 55
monster.race = "blood"
monster.speed = 1000
monster.manaCost = 220
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = false,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 7,
	runHealth = 55,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Chchch", yell = false},
}

monster.loot = {
	{id = 11100, chance = 100000}, -- flask with beaver bait
	{id = 7910, chance = 4550}, -- peanut
}

monster.defenses = {
	defense = 5,
	armor = 5,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}


mType:register(monster)