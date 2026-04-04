local mType = Game.createMonsterType("Teleskor")
local monster = {}

monster.description = "Teleskor"
monster.experience = 70
monster.outfit = {
	lookType = 298,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5972
monster.health = 80
monster.maxHealth = 80
monster.race = "undead"
monster.speed = 150
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 5
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Who Disturbs my slumber?", yell = false},
	{text = "Mourn the dead, do not hunt them!", yell = false},
}

monster.loot = {
	{id = 12437, chance = 100000}, -- pelvis bone
	{id = 2148, chance = 81000, maxCount = 79}, -- gold coin
	{id = 2398, chance = 72000}, -- mace
	{id = 2473, chance = 72000}, -- viking helmet
	{id = 2511, chance = 45000}, -- brass shield
	{id = 2050, chance = 36000}, -- torch
	{id = 2388, chance = 27000}, -- hatchet
	{id = 2376, chance = 27000}, -- sword
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}


mType:register(monster)