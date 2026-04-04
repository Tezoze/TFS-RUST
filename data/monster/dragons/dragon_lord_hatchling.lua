local mType = Game.createMonsterType("Dragon Lord Hatchling")
local monster = {}

monster.description = "a dragon lord hatchling"
monster.experience = 645
monster.outfit = {
	lookType = 272,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7622
monster.health = 750
monster.maxHealth = 750
monster.race = "blood"
monster.speed = 168
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
	staticAttackChance = 90,
	runHealth = 30,
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
	{id = 2148, chance = 33750, maxCount = 90}, -- gold coin
	{id = 2148, chance = 33750, maxCount = 75}, -- gold coin
	{id = 2672, chance = 80000}, -- dragon ham
	{id = 2796, chance = 560}, -- green mushroom
	{id = 7620, chance = 300}, -- mana potion
	{id = 7891, chance = 100}, -- magma boots
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -90, maxDamage = -125, effect = CONST_ME_FIREAREA, target = false, length = 5, spread = 2, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -55, maxDamage = -105, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 2000, chance = 10, range = 7, radius = 3, shootEffect = CONST_ANI_FIRE, target = true},
}

monster.defenses = {
	defense = 20,
	armor = 20,
	{name = "combat", interval = 2000, chance = 15, minDamage = 26, maxDamage = 48, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)