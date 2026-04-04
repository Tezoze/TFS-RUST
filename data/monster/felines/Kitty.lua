local mType = Game.createMonsterType("Kitty")
local monster = {}

monster.description = "a Kitty"
monster.experience = 40
monster.outfit = {
	lookType = 125,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6051
monster.health = 75
monster.maxHealth = 75
monster.race = "blood"
monster.speed = 200
monster.manaCost = 420
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = false,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2666, chance = 35190, maxCount = 4}, -- meat
	{id = 11210, chance = 10830}, -- striped fur
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -40, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 200, duration = 5000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -1},
}


mType:register(monster)