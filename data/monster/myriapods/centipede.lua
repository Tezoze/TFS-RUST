local mType = Game.createMonsterType("Centipede")
local monster = {}

monster.description = ""
monster.experience = 34
monster.outfit = {
	lookType = 124,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6050
monster.health = 70
monster.maxHealth = 70
monster.race = "venom"
monster.speed = 166
monster.manaCost = 335
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
	{id = 2148, chance = 80000, maxCount = 15}, -- gold coin
	{id = 11218, chance = 10300}, -- centipede leg
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false, condition = {type = CONDITION_POISON, startDamage = 20, interval = 2000}},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)