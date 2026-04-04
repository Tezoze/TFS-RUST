local mType = Game.createMonsterType("Old Wolf")
local monster = {}

monster.description = "a wolf"
monster.experience = 18
monster.outfit = {
	lookType = 918,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5968
monster.health = 25
monster.maxHealth = 25
monster.race = "blood"
monster.speed = 164
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
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

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Yoooohhuuuu!", yell = false},
	{text = "Grrrrrrr", yell = false},
}

monster.loot = {
	{id = 2666, chance = 55000, maxCount = 4}, -- meat
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 1,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 5},
	{type = COMBAT_HOLYDAMAGE, percent = 5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)