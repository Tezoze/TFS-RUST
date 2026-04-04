local mType = Game.createMonsterType("Boar")
local monster = {}

monster.description = "a boar"
monster.experience = 60
monster.outfit = {
	lookType = 380,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13308
monster.health = 198
monster.maxHealth = 198
monster.race = "blood"
monster.speed = 205
monster.manaCost = 465
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
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 30,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grunt! Grunt!", yell = false},
	{text = "Grunt", yell = false},
}

monster.loot = {
	{id = 2148, chance = 25000, maxCount = 20}, -- gold coin
	{id = 13297, chance = 20000, maxCount = 2}, -- haunch of boar
}

monster.attacks = {
	{name = "melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -50, target = false},
}

monster.defenses = {
	defense = 35,
	armor = 10,
}

monster.elements = {
}

monster.immunities = {
}


mType:register(monster)
