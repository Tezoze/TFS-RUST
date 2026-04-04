local mType = Game.createMonsterType("Tormented Ghost")
local monster = {}

monster.description = "a ghost"
monster.experience = 5
monster.outfit = {
	lookType = 48,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9741
monster.health = 210
monster.maxHealth = 210
monster.race = "undead"
monster.speed = 160
monster.manaCost = 100
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

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Haaahhh", yell = false},
	{text = "Grrglll", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -280, target = false},
	{name = "combat", interval = 3000, chance = 15, minDamage = -55, maxDamage = -105, range = 1, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 5,
	armor = 10,
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "physical", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)