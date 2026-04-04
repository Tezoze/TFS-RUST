local mType = Game.createMonsterType("Larva")
local monster = {}

monster.description = "a larva"
monster.experience = 44
monster.outfit = {
	lookType = 82,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6023
monster.health = 70
monster.maxHealth = 70
monster.race = "venom"
monster.speed = 124
monster.manaCost = 355
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 63000, maxCount = 15}, -- gold coin
	{id = 2666, chance = 14666}, -- meat
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false, condition = {type = CONDITION_POISON, startDamage = 15, interval = 2000}},
}

monster.defenses = {
	defense = 10,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)