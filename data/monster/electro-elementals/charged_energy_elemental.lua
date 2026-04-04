local mType = Game.createMonsterType("Charged Energy Elemental")
local monster = {}

monster.description = "a charged energy elemental"
monster.experience = 450
monster.outfit = {
	lookType = 293,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8966
monster.health = 500
monster.maxHealth = 500
monster.race = "undead"
monster.speed = 270
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
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 22}, -- gold coin
	{id = 7838, chance = 6250, maxCount = 3}, -- flash arrow
	{id = 8303, chance = 2063}, -- energy soil
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -100, interval = 2000, target = false},
	{name = "combat", type = COMBAT_ENERGYDAMAGE, minDamage = -168, maxDamage = -100, interval = 2000, chance = 20, range = 6, radius = 4, target = true, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_PURPLEENERGY},
	{name = "condition", type = CONDITION_ENERGY, interval = 1000, chance = 15, tick = 10000, minDamage = -25, maxDamage = -25, duration = 40000, radius = 3, effect = CONST_ME_YELLOWENERGY, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 2000, chance = 15, minDamage = 90, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)