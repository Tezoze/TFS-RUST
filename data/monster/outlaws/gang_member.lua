local mType = Game.createMonsterType("Gang Member")
local monster = {}

monster.description = "a gang member"
monster.experience = 70
monster.outfit = {
	lookType = 151,
	lookHead = 114,
	lookBody = 19,
	lookLegs = 42,
	lookFeet = 20,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 295
monster.maxHealth = 295
monster.race = "blood"
monster.speed = 190
monster.manaCost = 420
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 35,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "This is our territory!", yell = false},
	{text = "Help me guys!", yell = false},
	{text = "I don't like the way you look!", yell = false},
	{text = "You're wearing the wrong colours!", yell = false},
	{text = "Don't mess with us!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50110, maxCount = 30}, -- gold coin
	{id = 2207, chance = 740}, -- melee ring
	{id = 2468, chance = 5220}, -- studded legs
	{id = 2649, chance = 15330}, -- leather legs
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)