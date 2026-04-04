local mType = Game.createMonsterType("Dragon Hatchling")
local monster = {}

monster.description = "a dragon hatchling"
monster.experience = 185
monster.outfit = {
	lookType = 271,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7621
monster.health = 380
monster.maxHealth = 380
monster.race = "blood"
monster.speed = 146
monster.manaCost = 0
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
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	runHealth = 20,
	staticAttackChance = 90,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Fchu?", yell = false},
	{text = "Rooawwrr", yell = false},
}

monster.loot = {
	{id = 2148, chance = 67500, maxCount = 55}, -- gold coin
	{id = 2672, chance = 61000}, -- dragon ham
	{id = 7618, chance = 400}, -- health potion
	{id = 12413, chance = 4300}, -- dragon's tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -55, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -60, maxDamage = -90, effect = CONST_ME_FIREAREA, target = false, length = 5, spread = 2, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -55, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 15,
	{name = "combat", interval = 2000, chance = 15, minDamage = 8, maxDamage = 33, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 75},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)