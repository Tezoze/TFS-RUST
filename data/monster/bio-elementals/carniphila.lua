local mType = Game.createMonsterType("Carniphila")
local monster = {}

monster.description = "a carniphila"
monster.experience = 150
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
monster.health = 255
monster.maxHealth = 255
monster.race = "venom"
monster.speed = 110
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 60000, maxCount = 40}, -- gold coin
	{id = 2686, chance = 890}, -- corncob
	{id = 2792, chance = 7692}, -- dark mushroom
	{id = 2802, chance = 446, maxCount = 2}, -- sling herb
	{id = 2804, chance = 880}, -- shadow herb
	{id = 7732, chance = 490}, -- seeds
	{id = 11217, chance = 4166}, -- carniphila seeds
	{id = 13298, chance = 110}, -- carrot on a stick
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false, condition = {type = CONDITION_POISON, startDamage = 100, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -95, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENBUBBLE, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 2000, chance = 15, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENBUBBLE, target = true, speed = -800, duration = 30000},
	{name = "combat", interval = 2000, chance = 10, minDamage = -40, maxDamage = -130, radius = 3, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 22,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 35},
	{type = COMBAT_FIREDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)