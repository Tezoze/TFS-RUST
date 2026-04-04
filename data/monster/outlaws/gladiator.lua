local mType = Game.createMonsterType("Gladiator")
local monster = {}

monster.description = "a gladiator"
monster.experience = 90
monster.outfit = {
	lookType = 131,
	lookHead = 78,
	lookBody = 3,
	lookLegs = 79,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 185
monster.maxHealth = 185
monster.race = "blood"
monster.speed = 196
monster.manaCost = 470
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You are no match for me!", yell = false},
	{text = "Feel my prowess.", yell = false},
	{text = "Fight!", yell = false},
	{text = "Take this!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 49500, maxCount = 30}, -- gold coin
	{id = 2376, chance = 12620}, -- sword
	{id = 2398, chance = 11160}, -- mace
	{id = 2458, chance = 5200}, -- chain helmet
	{id = 2459, chance = 590}, -- iron helmet
	{id = 2465, chance = 2750}, -- brass armor
	{id = 2509, chance = 840}, -- steel shield
	{id = 2510, chance = 9950}, -- plate shield
	{id = 2666, chance = 19000}, -- meat
	{id = 8872, chance = 340}, -- belted cape
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 14,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 215, duration = 5000},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)