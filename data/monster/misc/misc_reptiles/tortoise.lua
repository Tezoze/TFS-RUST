local mType = Game.createMonsterType("Tortoise")
local monster = {}

monster.description = "a tortoise"
monster.experience = 90
monster.outfit = {
	lookType = 197,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6072
monster.health = 185
monster.maxHealth = 185
monster.race = "blood"
monster.speed = 200
monster.manaCost = 445
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 59000, maxCount = 30}, -- gold coin
	{id = 2417, chance = 730}, -- battle hammer
	{id = 2510, chance = 2850}, -- plate shield
	{id = 2667, chance = 4600}, -- fish
	{id = 5678, chance = 770, maxCount = 2}, -- tortoise egg
	{id = 5899, chance = 3300}, -- turtle shell
	{id = 6131, chance = 200}, -- tortoise shield
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 22,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 35},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}


mType:register(monster)