local mType = Game.createMonsterType("Tiquandas Revenge")
local monster = {}

monster.description = "Tiquandas Revenge"
monster.experience = 2635
monster.outfit = {
	lookType = 120,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6047
monster.health = 1800
monster.maxHealth = 1800
monster.race = "venom"
monster.speed = 440
monster.manaCost = 0
monster.maxSummons = 0

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
	runHealth = 400,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 10}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2666, chance = 33333, maxCount = 50}, -- meat
	{id = 2671, chance = 20000, maxCount = 8}, -- ham
	{id = 2145, chance = 33333, maxCount = 3}, -- small diamond
	{id = 7732, chance = 4000}, -- seeds
	{id = 5015, chance = 100000}, -- mandrake
	{id = 13298, chance = 12240}, -- carrot on a stick
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 70, attack = 40, target = false, condition = {type = CONDITION_POISON, startDamage = 95, interval = 2000}},
	{name = "combat", interval = 1000, chance = 25, minDamage = -60, maxDamage = -200, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENSPARK, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 1000, chance = 34, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENSPARK, target = true, speed = -850, duration = 30000},
	{name = "combat", interval = 1000, chance = 12, minDamage = -40, maxDamage = -130, radius = 3, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 31,
	armor = 30,
	{name = "combat", interval = 1200, chance = 35, minDamage = 100, maxDamage = 200, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)