local mType = Game.createMonsterType("Gozzler")
local monster = {}

monster.description = "a gozzler"
monster.experience = 180
monster.outfit = {
	lookType = 313,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9938
monster.health = 240
monster.maxHealth = 240
monster.race = "undead"
monster.speed = 240
monster.manaCost = 800
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
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Huhuhuhuuu!", yell = false},
	{text = "Let the fun begin!", yell = false},
	{text = "Yihahaha!", yell = false},
	{text = "I'll bite you! Nyehehehe!", yell = false},
	{text = "Nyarnyarnyarnyar.", yell = false},
}

monster.loot = {
	{id = 2015, chance = 8750}, -- brown flask
	{id = 2146, chance = 360}, -- small sapphire
	{id = 2148, chance = 52500, maxCount = 70}, -- gold coin
	{id = 2213, chance = 190}, -- dwarven ring
	{id = 2378, chance = 3100}, -- battle axe
	{id = 2385, chance = 8250}, -- sabre
	{id = 2394, chance = 5000}, -- morning star
	{id = 2409, chance = 250}, -- serpent sword
	{id = 2423, chance = 900}, -- clerical mace
	{id = 2510, chance = 10000}, -- plate shield
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -32, maxDamage = -135, range = 1, effect = CONST_ME_REDSPARK, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 25,
	{name = "combat", interval = 2000, chance = 10, minDamage = 30, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 210, duration = 5000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)