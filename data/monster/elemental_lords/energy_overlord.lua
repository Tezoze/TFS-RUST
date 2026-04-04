local mType = Game.createMonsterType("Energy Overlord")
local monster = {}

monster.description = "an Energy Overlord"
monster.experience = 2800
monster.outfit = {
	lookType = 290,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8966
monster.health = 4000
monster.maxHealth = 4000
monster.race = "undead"
monster.speed = 290
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 20000,
	chance = 15
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 85,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 64}, -- gold coin
	{id = 2152, chance = 25000, maxCount = 2}, -- platinum coin
	{id = 8306, chance = 100000}, -- pure energy
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = 0, maxDamage = -800, effect = CONST_ME_PURPLEENERGY, target = false, length = 7, spread = 0, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 11, minDamage = 0, maxDamage = -200, range = 3, effect = CONST_ME_PURPLEENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 9, minDamage = -50, maxDamage = -200, radius = 5, effect = CONST_ME_BIGPLANTS, target = false, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 40,
	{name = "combat", interval = 2000, chance = 15, minDamage = 90, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
}


mType:register(monster)