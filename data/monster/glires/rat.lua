local mType = Game.createMonsterType("Rat")
local monster = {}

monster.description = "a rat"
monster.experience = 5
monster.outfit = {
	lookType = 21,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5964
monster.health = 20
monster.maxHealth = 20
monster.race = "blood"
monster.speed = 134
monster.manaCost = 200
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
	runHealth = 5,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 70000, maxCount = 4}, -- gold coin
	{id = 2696, chance = 40056}, -- cheese
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -10, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 1,
}


mType:register(monster)