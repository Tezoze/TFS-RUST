local mType = Game.createMonsterType("Tarantula")
local monster = {}

monster.description = "a tarantula"
monster.experience = 120
monster.outfit = {
	lookType = 219,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6060
monster.health = 225
monster.maxHealth = 225
monster.race = "venom"
monster.speed = 214
monster.manaCost = 485
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 78590, maxCount = 40}, -- gold coin
	{id = 11198, chance = 10360}, -- tarantula egg
	{id = 2478, chance = 2800}, -- brass legs
	{id = 2510, chance = 1960}, -- plate shield
	{id = 2457, chance = 970}, -- steel helmet
	{id = 2169, chance = 160}, -- time ring
	{id = 8859, chance = 50}, -- spider fangs
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false, condition = {type = CONDITION_POISON, startDamage = 40, interval = 2000}},
	{name = "combat", interval = 2000, chance = 10, range = 1, radius = 1, shootEffect = CONST_ANI_POISON, effect = CONST_ME_CARNIPHILA, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 20,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 220, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -15},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)