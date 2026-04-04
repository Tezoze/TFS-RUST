local mType = Game.createMonsterType("Orc Berserker")
local monster = {}

monster.description = "an orc berserker"
monster.experience = 195
monster.outfit = {
	lookType = 8,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5980
monster.health = 210
monster.maxHealth = 210
monster.race = "blood"
monster.speed = 250
monster.manaCost = 590
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "KRAK ORRRRRRK!", yell = true},
}

monster.loot = {
	{id = 2044, chance = 830}, -- lamp
	{id = 2148, chance = 54000, maxCount = 12}, -- gold coin
	{id = 2378, chance = 6110}, -- battle axe
	{id = 2381, chance = 7280}, -- halberd
	{id = 2464, chance = 890}, -- chain armor
	{id = 2671, chance = 10400}, -- ham
	{id = 3965, chance = 5000}, -- hunting spear
	{id = 11113, chance = 3000}, -- orc tooth
	{id = 12433, chance = 9400}, -- orcish gear
	{id = 12435, chance = 4000}, -- orc leather
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 12,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 290, duration = 6000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 15},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)