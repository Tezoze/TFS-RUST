local mType = Game.createMonsterType("Lion")
local monster = {}

monster.description = "a lion"
monster.experience = 30
monster.outfit = {
	lookType = 41,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5986
monster.health = 80
monster.maxHealth = 80
monster.race = "blood"
monster.speed = 190
monster.manaCost = 320
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
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Groarrr!", yell = false},
}

monster.loot = {
	{id = 2666, chance = 45000, maxCount = 4}, -- meat
	{id = 2671, chance = 18430, maxCount = 2}, -- ham
	{id = 10608, chance = 1400}, -- lion's mane
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -40, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -8},
}


mType:register(monster)