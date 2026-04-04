local mType = Game.createMonsterType("Brutus Bloodbeard")
local monster = {}

monster.description = "Brutus Bloodbeard"
monster.experience = 795
monster.outfit = {
	lookType = 98,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 1555
monster.maxHealth = 1555
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
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
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 6099, chance = 100000}, -- brutus bloodbeard's hat
	{id = 2148, chance = 100000, maxCount = 200}, -- gold coin
	{id = 2229, chance = 75000, maxCount = 2}, -- skull
	{id = 2379, chance = 25000}, -- dagger
	{id = 2476, chance = 25000}, -- knight armor
	{id = 2666, chance = 25000}, -- meat
	{id = 2463, chance = 25000}, -- plate armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -175, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -175, range = 7, shootEffect = CONST_ANI_THROWINGSTAR, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, effect = CONST_ME_POFF, target = false, length = 3, spread = 2, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 50,
	armor = 35,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -1},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_ICEDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)