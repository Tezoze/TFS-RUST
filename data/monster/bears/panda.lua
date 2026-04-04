local mType = Game.createMonsterType("Panda")
local monster = {}

monster.description = "a panda"
monster.experience = 23
monster.outfit = {
	lookType = 123,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6049
monster.health = 80
monster.maxHealth = 80
monster.race = "blood"
monster.speed = 156
monster.manaCost = 300
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
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrr", yell = false},
	{text = "Groar", yell = false},
}

monster.loot = {
	{id = 2666, chance = 70500, maxCount = 4}, -- meat
	{id = 2671, chance = 39000, maxCount = 2}, -- ham
	{id = 12401, chance = 10000}, -- bamboo stick
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)