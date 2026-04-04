local mType = Game.createMonsterType("Crab")
local monster = {}

monster.description = ""
monster.experience = 30
monster.outfit = {
	lookType = 112,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6039
monster.health = 55
monster.maxHealth = 55
monster.race = "undead"
monster.speed = 144
monster.manaCost = 305
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
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
	{id = 2148, chance = 80000, maxCount = 10}, -- gold coin
	{id = 2667, chance = 20000}, -- fish
	{id = 11189, chance = 20000}, -- crab pincers
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 1},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)