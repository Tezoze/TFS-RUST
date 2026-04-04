local mType = Game.createMonsterType("Squirrel")
local monster = {}

monster.description = "a squirrel"
monster.experience = 0
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
monster.health = 20
monster.maxHealth = 20
monster.race = "blood"
monster.speed = 240
monster.manaCost = 480
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
	runHealth = 20,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Chchch", yell = false},
}

monster.loot = {
	{id = 7909, chance = 1140}, -- walnut
	{id = 7910, chance = 980}, -- peanut
	{id = 11213, chance = 50410}, -- acorn
}

monster.defenses = {
	defense = 5,
	armor = 1,
}


mType:register(monster)