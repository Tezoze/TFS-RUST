local mType = Game.createMonsterType("Fleabringer")
local monster = {}

monster.description = "a fleabringer"
monster.experience = 100
monster.outfit = {
	lookType = 341,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11250
monster.health = 265
monster.maxHealth = 265
monster.race = "blood"
monster.speed = 280
monster.manaCost = 465
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2666, chance = 25000, maxCount = 3}, -- meat
	{id = 3976, chance = 75000, maxCount = 3}, -- worm
	{id = 11324, chance = 99990}, -- shaggy tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)