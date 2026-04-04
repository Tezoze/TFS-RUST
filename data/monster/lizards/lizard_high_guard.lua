local mType = Game.createMonsterType("Lizard High Guard")
local monster = {}

monster.description = "a lizard high guard"
monster.experience = 1305
monster.outfit = {
	lookType = 337,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11272
monster.health = 1800
monster.maxHealth = 1800
monster.race = "blood"
monster.speed = 238
monster.manaCost = 0
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
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hizzzzzzz!", yell = false},
	{text = "To armzzzz!", yell = false},
	{text = "Engage zze aggrezzor!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 96080, maxCount = 281}, -- gold coin
	{id = 7588, chance = 11940}, -- strong health potion
	{id = 11333, chance = 8120}, -- high guard shoulderplates
	{id = 7591, chance = 7070}, -- great health potion
	{id = 11325, chance = 6980}, -- spiked iron ball
	{id = 11245, chance = 4920}, -- bunch of ripe rice
	{id = 2152, chance = 4920, maxCount = 2}, -- platinum coin
	{id = 11332, chance = 3000}, -- high guard flag
	{id = 2149, chance = 2490, maxCount = 4}, -- small emerald
	{id = 11206, chance = 1200}, -- red lantern
	{id = 2528, chance = 1030}, -- tower shield
	{id = 5876, chance = 1000}, -- lizard leather
	{id = 5881, chance = 960}, -- lizard scale
	{id = 11304, chance = 730}, -- Zaoan legs
	{id = 11303, chance = 690}, -- Zaoan shoes
	{id = 11301, chance = 80}, -- Zaoan armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -261, target = false},
}

monster.defenses = {
	defense = 35,
	armor = 40,
	{name = "combat", interval = 2000, chance = 10, minDamage = 25, maxDamage = 75, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = 45},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)