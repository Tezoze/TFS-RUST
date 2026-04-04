local mType = Game.createMonsterType("Hide")
local monster = {}

monster.description = "Hide"
monster.experience = 240
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
monster.health = 500
monster.maxHealth = 500
monster.race = "venom"
monster.speed = 280
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
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2152, chance = 79170, maxCount = 5}, -- platinum coin
	{id = 5879, chance = 63890}, -- spider silk
	{id = 2457, chance = 61810}, -- steel helmet
	{id = 2477, chance = 51390}, -- knight legs
	{id = 7903, chance = 37500}, -- terra hood
	{id = 2169, chance = 34030}, -- time ring
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 60, attack = 40, target = false, condition = {type = CONDITION_POISON, startDamage = 80, interval = 2000}},
}

monster.defenses = {
	defense = 40,
	armor = 25,
	{name = "speed", interval = 2000, chance = 10, effect = CONST_ME_REDSHIMMER, speed = 340, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -15},
	{type = COMBAT_ICEDAMAGE, percent = -15},
	{type = COMBAT_PHYSICALDAMAGE, percent = 40},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)