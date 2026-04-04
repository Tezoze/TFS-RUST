local mType = Game.createMonsterType("Skunk")
local monster = {}

monster.description = "a skunk"
monster.experience = 3
monster.outfit = {
	lookType = 106,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6035
monster.health = 20
monster.maxHealth = 20
monster.race = "blood"
monster.speed = 120
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
	runHealth = 8,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 9114, chance = 4910}, -- bulb of garlic
	{id = 11191, chance = 920}, -- skunk tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -5, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -1, maxDamage = -3, range = 1, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 5,
	armor = 1,
}


mType:register(monster)