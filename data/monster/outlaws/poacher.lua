local mType = Game.createMonsterType("Poacher")
local monster = {}

monster.description = "a poacher"
monster.experience = 70
monster.outfit = {
	lookType = 129,
	lookHead = 115,
	lookBody = 119,
	lookLegs = 119,
	lookFeet = 115,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 90
monster.maxHealth = 90
monster.race = "blood"
monster.speed = 198
monster.manaCost = 530
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 5,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You will not live to tell anyone!", yell = false},
	{text = "You are my game today!", yell = false},
	{text = "Look what has stepped into my trap!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 4180}, -- torch
	{id = 2456, chance = 14930}, -- bow
	{id = 2461, chance = 30600}, -- leather helmet
	{id = 2544, chance = 49500, maxCount = 17}, -- arrow
	{id = 2545, chance = 2930, maxCount = 3}, -- poison arrow
	{id = 2578, chance = 710}, -- closed trap
	{id = 2649, chance = 26740}, -- leather legs
	{id = 2690, chance = 11110, maxCount = 2}, -- roll
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -35, range = 7, shootEffect = CONST_ANI_ARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 10,
}


mType:register(monster)