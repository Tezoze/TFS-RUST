local mType = Game.createMonsterType("Lethal Lissy")
local monster = {}

monster.description = "Lethal Lissy"
monster.experience = 500
monster.outfit = {
	lookType = 155,
	lookHead = 77,
	lookBody = 0,
	lookLegs = 76,
	lookFeet = 132,
	lookAddons = 3,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 1450
monster.maxHealth = 1450
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 50,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2145, chance = 12500}, -- small diamond
	{id = 2666, chance = 18750, maxCount = 3}, -- meat
	{id = 2148, chance = 50000, maxCount = 60}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 61}, -- gold coin
	{id = 2229, chance = 81250, maxCount = 2}, -- skull
	{id = 5926, chance = 6250}, -- pirate backpack
	{id = 2463, chance = 56250}, -- plate armor
	{id = 2476, chance = 12500}, -- knight armor
	{id = 10103, chance = 25000}, -- very old piece of paper
	{id = 6100, chance = 100000}, -- lethal lissy's shirt
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
}

monster.defenses = {
	defense = 50,
	armor = 35,
	{name = "combat", interval = 6000, chance = 65, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Pirate Cutthroat", chance = 50, interval = 2000, max = 4},
}

mType:register(monster)