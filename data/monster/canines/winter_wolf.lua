local mType = Game.createMonsterType("Winter Wolf")
local monster = {}

monster.description = "a winter wolf"
monster.experience = 20
monster.outfit = {
	lookType = 52,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5997
monster.health = 30
monster.maxHealth = 30
monster.race = "blood"
monster.speed = 170
monster.manaCost = 260
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
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Yoooohhuuuu!", yell = false},
}

monster.loot = {
	{id = 2666, chance = 30000, maxCount = 2}, -- meat
	{id = 11212, chance = 10000}, -- winter wolf fur
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 2,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 5},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)