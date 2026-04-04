local mType = Game.createMonsterType("Wasp")
local monster = {}

monster.description = "a wasp"
monster.experience = 24
monster.outfit = {
	lookType = 44,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5989
monster.health = 35
monster.maxHealth = 35
monster.race = "venom"
monster.speed = 320
monster.manaCost = 280
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

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Bssssss", yell = false},
}

monster.loot = {
	{id = 5902, chance = 3000, maxCount = 3}, -- honeycomb
}

monster.attacks = {
	{name = "melee", interval = 1500, minDamage = 0, maxDamage = -20, target = false, condition = {type = CONDITION_POISON, startDamage = 20, interval = 2000}},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)