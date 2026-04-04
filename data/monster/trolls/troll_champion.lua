local mType = Game.createMonsterType("Troll Champion")
local monster = {}

monster.description = "a troll champion"
monster.experience = 40
monster.outfit = {
	lookType = 281,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7926
monster.health = 75
monster.maxHealth = 75
monster.race = "blood"
monster.speed = 138
monster.manaCost = 340
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
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Meee maity!", yell = false},
	{text = "Grrrr", yell = false},
	{text = "Whaaaz up!?", yell = false},
	{text = "Gruntz!", yell = false},
	{text = "Groar", yell = false},
}

monster.loot = {
	{id = 2148, chance = 64000, maxCount = 12}, -- gold coin
	{id = 2170, chance = 230}, -- silver amulet
	{id = 2389, chance = 25000}, -- spear
	{id = 2448, chance = 5450}, -- studded club
	{id = 2512, chance = 6000}, -- wooden shield
	{id = 2544, chance = 5450, maxCount = 5}, -- arrow
	{id = 2643, chance = 9000}, -- leather boots
	{id = 2666, chance = 9650}, -- meat
	{id = 10606, chance = 3000}, -- bunch of troll hair
	{id = 12471, chance = 750}, -- trollroot
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 15},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)