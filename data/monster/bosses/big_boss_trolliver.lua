local mType = Game.createMonsterType("Big Boss Trolliver")
local monster = {}

monster.description = "Big Boss Trolliver"
monster.experience = 105
monster.outfit = {
	lookType = 281,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7926
monster.health = 150
monster.maxHealth = 150
monster.race = "blood"
monster.speed = 190
monster.manaCost = 0
monster.maxSummons = 5

monster.changeTarget = {
	interval = 2000,
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
	canPushCreatures = false,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 64}, -- gold coin
	{id = 2389, chance = 25000}, -- spear
	{id = 2666, chance = 9650, maxCount = 3}, -- meat
	{id = 2643, chance = 9000}, -- leather boots
	{id = 2448, chance = 5450}, -- studded club
	{id = 2170, chance = 100000}, -- silver amulet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 15},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Troll Champion", chance = 30, interval = 2000, max = 5},
}

mType:register(monster)