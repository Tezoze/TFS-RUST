local mType = Game.createMonsterType("Rottie The Rotworm")
local monster = {}

monster.description = "Rottie The Rotworm"
monster.experience = 40
monster.outfit = {
	lookType = 26,
	lookHead = 20,
	lookBody = 30,
	lookLegs = 40,
	lookFeet = 50,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5967
monster.health = 65
monster.maxHealth = 65
monster.race = "blood"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 6230, maxCount = 30}, -- gold coin
	{id = 2530, chance = 2850}, -- copper shield
	{id = 2666, chance = 3260, maxCount = 2}, -- meat
	{id = 3976, chance = 32500, maxCount = 5}, -- worm
	{id = 2398, chance = 3335}, -- mace
	{id = 2671, chance = 3160, maxCount = 2}, -- ham
	{id = 2376, chance = 3335}, -- sword
	{id = 2412, chance = 900}, -- katana
	{id = 2480, chance = 1250}, -- legion helmet
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 30, attack = 30, target = false},
}

monster.defenses = {
	defense = 11,
	armor = 8,
}


mType:register(monster)