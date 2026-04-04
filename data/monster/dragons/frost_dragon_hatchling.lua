local mType = Game.createMonsterType("Frost Dragon Hatchling")
local monster = {}

monster.description = "a frost dragon hatchling"
monster.experience = 745
monster.outfit = {
	lookType = 283,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7969
monster.health = 800
monster.maxHealth = 800
monster.race = "undead"
monster.speed = 172
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
	runHealth = 80,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Rooawwrr", yell = false},
	{text = "Fchu?", yell = false},
}

monster.loot = {
	{id = 2148, chance = 86750, maxCount = 55}, -- gold coin
	{id = 2672, chance = 79600}, -- dragon ham
	{id = 7618, chance = 560}, -- health potion
	{id = 8900, chance = 400}, -- spellbook of enlightenment
	{id = 10578, chance = 5000}, -- frosty heart
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -160, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -60, maxDamage = -110, effect = CONST_ME_ICEATTACK, target = false, length = 5, spread = 2, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -110, radius = 4, effect = CONST_ME_ICEAREA, target = true, type = COMBAT_ICEDAMAGE},
	{name = "speed", interval = 2000, chance = 15, radius = 4, shootEffect = CONST_ANI_ICE, effect = CONST_ME_ICEAREA, target = true, speed = -600, duration = 12000},
}

monster.defenses = {
	defense = 15,
	armor = 32,
	{name = "combat", interval = 2000, chance = 15, minDamage = 45, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)