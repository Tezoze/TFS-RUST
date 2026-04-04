local mType = Game.createMonsterType("Coral Frog")
local monster = {}

monster.description = ""
monster.experience = 20
monster.outfit = {
	lookType = 226,
	lookHead = 114,
	lookBody = 98,
	lookLegs = 97,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6079
monster.health = 60
monster.maxHealth = 60
monster.race = "blood"
monster.speed = 320
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
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ribbit!", yell = false},
	{text = "Ribbit! Ribbit!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 76780, maxCount = 10}, -- gold coin
	{id = 3976, chance = 13510}, -- worm
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -24, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}


mType:register(monster)