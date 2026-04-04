local mType = Game.createMonsterType("Troll")
local monster = {}

monster.description = "a troll"
monster.experience = 20
monster.outfit = {
	lookType = 15,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5960
monster.health = 50
monster.maxHealth = 50
monster.race = "blood"
monster.speed = 126
monster.manaCost = 290
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
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrr", yell = false},
	{text = "Groar", yell = false},
	{text = "Gruntz!", yell = false},
	{text = "Hmmm, bugs", yell = false},
	{text = "Hmmm, dogs", yell = false},
}

monster.loot = {
	{id = 2148, chance = 65090, maxCount = 12}, -- gold coin
	{id = 2380, chance = 17870}, -- hand axe
	{id = 2666, chance = 15150}, -- meat
	{id = 2389, chance = 13150}, -- spear
	{id = 2461, chance = 11780}, -- leather helmet
	{id = 2643, chance = 10020}, -- leather boots
	{id = 2120, chance = 7790}, -- rope
	{id = 2448, chance = 5020}, -- studded club
	{id = 2512, chance = 5000}, -- wooden shield
	{id = 10606, chance = 1020}, -- bunch of troll hair
	{id = 2170, chance = 100}, -- silver amulet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -15, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 6,
}


mType:register(monster)