local mType = Game.createMonsterType("Skeleton")
local monster = {}

monster.description = "a skeleton"
monster.experience = 35
monster.outfit = {
	lookType = 33,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5972
monster.health = 50
monster.maxHealth = 50
monster.race = "undead"
monster.speed = 154
monster.manaCost = 300
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2050, chance = 10000}, -- torch
	{id = 2050, chance = 9880}, -- torch
	{id = 2148, chance = 43900, maxCount = 10}, -- gold coin
	{id = 2230, chance = 49100}, -- bone
	{id = 2376, chance = 1940}, -- sword
	{id = 2388, chance = 4850}, -- hatchet
	{id = 2398, chance = 4850}, -- mace
	{id = 2473, chance = 7520}, -- viking helmet
	{id = 2511, chance = 2090}, -- brass shield
	{id = 12437, chance = 9940}, -- pelvis bone
	{id = 1950, chance = 1000}, -- book
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -7, maxDamage = -13, range = 1, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 10,
	armor = 2,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)