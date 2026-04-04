local mType = Game.createMonsterType("Elephant")
local monster = {}

monster.description = "an elephant"
monster.experience = 160
monster.outfit = {
	lookType = 211,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6052
monster.health = 320
monster.maxHealth = 320
monster.race = "blood"
monster.speed = 190
monster.manaCost = 500
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hooooot-Toooooot!", yell = false},
	{text = "Tooooot!", yell = false},
	{text = "Trooooot!", yell = false},
}

monster.loot = {
	{id = 2666, chance = 39000, maxCount = 4}, -- meat
	{id = 2671, chance = 30000, maxCount = 3}, -- ham
	{id = 3956, chance = 10000, maxCount = 2}, -- tusk
	{id = 3956, chance = 1000, maxCount = 2}, -- tusk
	{id = 3973, chance = 140}, -- tusk shield
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}


mType:register(monster)