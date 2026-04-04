local mType = Game.createMonsterType("The Old Widow")
local monster = {}

monster.description = "The Old Widow"
monster.experience = 4200
monster.outfit = {
	lookType = 208,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5977
monster.health = 3200
monster.maxHealth = 3200
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 2

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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 99}, -- gold coin
	{id = 2152, chance = 100000, maxCount = 10}, -- platinum coin
	{id = 5879, chance = 100000}, -- spider silk
	{id = 2457, chance = 100000}, -- steel helmet
	{id = 7591, chance = 100000, maxCount = 4}, -- great health potion
	{id = 2476, chance = 50000}, -- knight armor
	{id = 2165, chance = 33333}, -- stealth ring
	{id = 2167, chance = 33333}, -- energy ring
	{id = 2169, chance = 33333}, -- time ring
	{id = 8297, chance = 33333}, -- bait
	{id = 2477, chance = 25000}, -- knight legs
	{id = 2171, chance = 25000}, -- platinum amulet
	{id = 5886, chance = 25000}, -- spool of yarn
	{id = 7416, chance = 3225}, -- bloody edge
	{id = 7419, chance = 1639}, -- dreaded cleaver
	{id = 13307, chance = 22030}, -- sweet smelling bait
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = -100, maxDamage = -500, target = false},
	{name = "combat", interval = 1000, chance = 15, minDamage = -250, maxDamage = -300, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 1000, chance = 20, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, speed = -850, duration = 25000},
	{name = "poisonfield", interval = 1000, chance = 10, range = 7, radius = 4, shootEffect = CONST_ANI_POISON, target = true},
}

monster.defenses = {
	defense = 21,
	armor = 45,
	{name = "combat", interval = 1000, chance = 17, minDamage = 225, maxDamage = 275, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 8, effect = CONST_ME_REDSHIMMER, speed = 345, duration = 6000},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "poison", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "giant spider", chance = 13, interval = 1000, max = 2},
}

mType:register(monster)