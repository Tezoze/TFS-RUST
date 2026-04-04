local mType = Game.createMonsterType("Draken Warmaster")
local monster = {}

monster.description = "a draken warmaster"
monster.experience = 2400
monster.outfit = {
	lookType = 334,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11107
monster.health = 4150
monster.maxHealth = 4150
monster.race = "blood"
monster.speed = 324
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Attack aggrezzively! Dezztroy zze intruderzz!", yell = false},
	{text = "Hizzzzzz!", yell = false},
}

monster.loot = {
	{id = 2123, chance = 180}, -- ring of the sky
	{id = 2147, chance = 1525, maxCount = 5}, -- small ruby
	{id = 2148, chance = 47000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 50625, maxCount = 5}, -- platinum coin
	{id = 2528, chance = 2035}, -- tower shield
	{id = 2666, chance = 30300}, -- meat
	{id = 7591, chance = 4850, maxCount = 3}, -- great health potion
	{id = 8473, chance = 4020}, -- ultimate health potion
	{id = 11301, chance = 790}, -- Zaoan armor
	{id = 11303, chance = 1900}, -- Zaoan shoes
	{id = 11304, chance = 960}, -- Zaoan legs
	{id = 11305, chance = 860}, -- drakinata
	{id = 11321, chance = 12010}, -- bone shoulderplate
	{id = 11322, chance = 7000}, -- warmaster's wristguards
	{id = 11323, chance = 7925}, -- Zaoan halberd
	{id = 11134, chance = 1000}, -- Tome of Knowledge
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -240, maxDamage = -520, effect = CONST_ME_EXPLOSION, target = false, length = 4, spread = 3, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 55,
	{name = "combat", interval = 2000, chance = 10, minDamage = 510, maxDamage = 600, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_HOLYDAMAGE, percent = 5},
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = 5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)