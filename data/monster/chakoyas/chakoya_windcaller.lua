local mType = Game.createMonsterType("Chakoya Windcaller")
local monster = {}

monster.description = "a chakoya windcaller"
monster.experience = 48
monster.outfit = {
	lookType = 260,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7320
monster.health = 84
monster.maxHealth = 84
monster.race = "blood"
monster.speed = 142
monster.manaCost = 305
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 90,
	targetDistance = 4,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Mupi! Si siyoqua jinuma!", yell = false},
	{text = "Siqsiq ji jusipa!", yell = false},
	{text = "Jagura taluka taqua!", yell = false},
	{text = "Quatu nguraka!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 71170, maxCount = 27}, -- gold coin
	{id = 2460, chance = 4390}, -- brass helmet
	{id = 2541, chance = 960}, -- bone shield
	{id = 2667, chance = 30790, maxCount = 3}, -- fish
	{id = 2669, chance = 2040}, -- northern pike
	{id = 7158, chance = 2040}, -- rainbow trout
	{id = 7159, chance = 2110}, -- green perch
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -22, interval = 2000, target = false},
	{name = "combat", type = COMBAT_ICEDAMAGE, minDamage = -16, maxDamage = -32, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_ICE},
	{name = "condition", type = CONDITION_FREEZING, interval = 2000, chance = 10, tick = 10000, minDamage = -130, maxDamage = -160, radius = 3, effect = CONST_ME_ICEAREA, target = false},
	{name = "combat", type = COMBAT_ICEDAMAGE, minDamage = -9, maxDamage = -30, interval = 2000, chance = 10, length = 5, spread = 2, target = false, effect = CONST_ME_ICEAREA},
}

monster.defenses = {
	defense = 10,
	armor = 7,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)