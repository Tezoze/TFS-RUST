local mType = Game.createMonsterType("Enraged Squirrel")
local monster = {}

monster.description = "an enraged squirrel"
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
monster.health = 35
monster.maxHealth = 35
monster.race = "blood"
monster.speed = 300
monster.manaCost = 220
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 2,
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
	{id = 7909, chance = 2680}, -- walnut
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 10, attack = 10, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 1,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}


mType:register(monster)