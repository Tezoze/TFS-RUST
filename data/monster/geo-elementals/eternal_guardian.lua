local mType = Game.createMonsterType("Eternal Guardian")
local monster = {}

monster.description = "an eternal guardian"
monster.experience = 1800
monster.outfit = {
	lookType = 345,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11300
monster.health = 2500
monster.maxHealth = 2500
monster.race = "undead"
monster.speed = 204
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Zzrrkrrch!", yell = false},
	{text = "<crackle>", yell = false},
}

monster.loot = {
	{id = 1294, chance = 30230, maxCount = 10}, -- small stone
	{id = 2148, chance = 99930, maxCount = 100}, -- gold coin
	{id = 2152, chance = 99540, maxCount = 4}, -- platinum coin
	{id = 2427, chance = 560}, -- guardian halberd
	{id = 2528, chance = 820}, -- tower shield
	{id = 5880, chance = 1700}, -- iron ore
	{id = 10549, chance = 20020}, -- ancient stone
	{id = 11227, chance = 800}, -- shiny stone
	{id = 11307, chance = 100}, -- Zaoan sword
	{id = 11323, chance = 1860}, -- Zaoan halberd
	{id = 11325, chance = 9960}, -- spiked iron ball
	{id = 11339, chance = 720}, -- clay lump
	{id = 11343, chance = 430}, -- piece of marble rock
	{id = 8748, chance = 400},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
}

monster.defenses = {
	defense = 40,
	armor = 62,
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 70},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)