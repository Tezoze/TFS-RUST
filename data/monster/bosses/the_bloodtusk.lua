local mType = Game.createMonsterType("The Bloodtusk")
local monster = {}

monster.description = "the Bloodtusk"
monster.experience = 300
monster.outfit = {
	lookType = 199,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6074
monster.health = 600
monster.maxHealth = 600
monster.race = "blood"
monster.speed = 190
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 99}, -- gold coin
	{id = 11238, chance = 100000, maxCount = 2}, -- mammoth tusk
	{id = 2152, chance = 100000, maxCount = 5}, -- platinum coin
	{id = 7432, chance = 63000}, -- furry club
	{id = 5911, chance = 60000}, -- red piece of cloth
	{id = 3973, chance = 55000}, -- tusk shield
	{id = 3956, chance = 41000, maxCount = 4}, -- tusk
	{id = 7463, chance = 18000}, -- mammoth fur cape
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -120, target = false},
}

monster.defenses = {
	defense = 57,
	armor = 40,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 15},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)