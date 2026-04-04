local mType = Game.createMonsterType("Dworc Voodoomaster")
local monster = {}

monster.description = ""
monster.experience = 55
monster.outfit = {
	lookType = 214,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6055
monster.health = 80
monster.maxHealth = 80
monster.race = "blood"
monster.speed = 150
monster.manaCost = 350
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
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
	targetDistance = 4,
	staticAttackChance = 80,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Brak brrretz!", yell = false},
	{text = "Grow truk grrrrr.", yell = false},
	{text = "Prek tars, dekklep zurk.", yell = false},
}

monster.loot = {
	{id = 2050, chance = 6000}, -- torch
	{id = 2148, chance = 75000, maxCount = 17}, -- gold coin
	{id = 2174, chance = 500}, -- strange symbol
	{id = 2229, chance = 1950, maxCount = 3}, -- skull
	{id = 2230, chance = 5800}, -- bone
	{id = 2231, chance = 3000}, -- big bone
	{id = 2411, chance = 1000}, -- poison dagger
	{id = 2467, chance = 10000}, -- leather armor
	{id = 3955, chance = 1130}, -- voodoo doll
	{id = 3967, chance = 1000}, -- tribal mask
	{id = 7618, chance = 600}, -- health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -40, range = 1, target = false, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 10, range = 7, target = false, speed = -800, duration = 5000},
	{name = "combat", interval = 2000, chance = 10, range = 7, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "outfit", interval = 2000, chance = 10, range = 7, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -6, maxDamage = -18, radius = 6, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "poisonfield", interval = 2000, chance = 10, range = 7, radius = 1, target = true},
}

monster.defenses = {
	defense = 10,
	armor = 10,
	{name = "combat", interval = 2000, chance = 20, minDamage = 3, maxDamage = 9, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, speed = 200, duration = 4000},
	{name = "invisible", interval = 4000, chance = 15},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_FIREDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)