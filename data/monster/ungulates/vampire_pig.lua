local mType = Game.createMonsterType("Vampire Pig")
local monster = {}

monster.description = "a vampire pig"
monster.experience = 165
monster.outfit = {
	lookType = 60,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6000
monster.health = 305
monster.maxHealth = 305
monster.race = "blood"
monster.speed = 110
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 30,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Oink", yell = false},
	{text = "Oink oink", yell = false},
}

monster.loot = {
	{id = 2148, chance = 90000, maxCount = 40}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = -15, maxDamage = -25, radius = 4, effect = CONST_ME_BATS, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -35, maxDamage = -55, range = 3, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 35,
	armor = 20,
	{name = "outfit", interval = 2000, chance = 20, effect = CONST_ME_BLUESHIMMER, monster = "mutated bat", duration = 1500},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)