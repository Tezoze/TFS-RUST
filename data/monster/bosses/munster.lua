local mType = Game.createMonsterType("Munster")
local monster = {}

monster.description = "Munster"
monster.experience = 35
monster.outfit = {
	lookType = 56,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 2813
monster.health = 58
monster.maxHealth = 58
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 10000,
	chance = 5
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 10,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Meep!", yell = false},
	{text = "Meeeeep!", yell = false},
}

monster.loot = {
	{id = 2449, chance = 87000}, -- bone club
	{id = 2148, chance = 71000, maxCount = 22}, -- gold coin
	{id = 2696, chance = 56000}, -- cheese
	{id = 3976, chance = 51000, maxCount = 4}, -- worm
	{id = 2687, chance = 2500, maxCount = 2}, -- cookie
	{id = 5792, chance = 250},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -15, target = false},
}

monster.defenses = {
	defense = 4,
	armor = 2,
}

monster.summons = {
	{name = "Rat", chance = 20, interval = 2000, max = 2},
}

mType:register(monster)