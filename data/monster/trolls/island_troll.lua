local mType = Game.createMonsterType("Island Troll")
local monster = {}

monster.description = "an island troll"
monster.experience = 20
monster.outfit = {
	lookType = 282,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7930
monster.health = 50
monster.maxHealth = 50
monster.race = "blood"
monster.speed = 126
monster.manaCost = 290
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hmmm, turtles", yell = false},
	{text = "Hmmm, dogs", yell = false},
	{text = "Hmmm, worms", yell = false},
	{text = "Groar", yell = false},
	{text = "Gruntz!", yell = false},
}

monster.loot = {
	{id = 2120, chance = 8000}, -- rope
	{id = 2148, chance = 60000, maxCount = 10}, -- gold coin
	{id = 2170, chance = 70}, -- silver amulet
	{id = 2380, chance = 18000}, -- hand axe
	{id = 2389, chance = 20000}, -- spear
	{id = 2448, chance = 5000}, -- studded club
	{id = 2461, chance = 10000}, -- leather helmet
	{id = 2512, chance = 16000}, -- wooden shield
	{id = 2643, chance = 10500}, -- leather boots
	{id = 5097, chance = 5000}, -- mango
	{id = 5901, chance = 30000}, -- wood
	{id = 7963, chance = 1040}, -- marlin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -10, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 6,
}


mType:register(monster)