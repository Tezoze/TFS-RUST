local mType = Game.createMonsterType("Flamingo")
local monster = {}

monster.description = "a flamingo"
monster.experience = 0
monster.outfit = {
	lookType = 212,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6054
monster.health = 25
monster.maxHealth = 25
monster.race = "blood"
monster.speed = 168
monster.manaCost = 250
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = false,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 0,
	runHealth = 25,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 12640, chance = 700}, -- downy feather
}

monster.defenses = {
	defense = 5,
	armor = 1,
}


mType:register(monster)