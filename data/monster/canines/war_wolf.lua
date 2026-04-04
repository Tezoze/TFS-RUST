local mType = Game.createMonsterType("War Wolf")
local monster = {}

monster.description = "a war wolf"
monster.experience = 55
monster.outfit = {
	lookType = 3,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6009
monster.health = 140
monster.maxHealth = 140
monster.race = "blood"
monster.speed = 264
monster.manaCost = 420
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrrrrr", yell = false},
	{text = "Yoooohhuuuu!", yell = true},
}

monster.loot = {
	{id = 2671, chance = 35000, maxCount = 2}, -- ham
	{id = 5897, chance = 5710}, -- wolf paw
	{id = 11235, chance = 5230}, -- warwolf fur
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
}


mType:register(monster)