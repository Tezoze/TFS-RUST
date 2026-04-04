local mType = Game.createMonsterType("Rotworm")
local monster = {}

monster.description = "a rotworm"
monster.experience = 40
monster.outfit = {
	lookType = 26,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5967
monster.health = 65
monster.maxHealth = 65
monster.race = "blood"
monster.speed = 116
monster.manaCost = 305
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 71760, maxCount = 17}, -- gold coin
	{id = 2376, chance = 3000}, -- sword
	{id = 2398, chance = 4500}, -- mace
	{id = 2666, chance = 20000}, -- meat
	{id = 2671, chance = 20120}, -- ham
	{id = 3976, chance = 3000, maxCount = 3}, -- worm
	{id = 10609, chance = 10000}, -- lump of dirt
	{id = 2480, chance = 1500}, -- legion helmet
	{id = 2530, chance = 1500}, -- copper shield
	{id = 2412, chance = 1500}, -- katana
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -40, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 8,
}


mType:register(monster)