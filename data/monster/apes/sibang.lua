local mType = Game.createMonsterType("Sibang")
local monster = {}

monster.description = "a sibang"
monster.experience = 105
monster.outfit = {
	lookType = 118,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6045
monster.health = 225
monster.maxHealth = 225
monster.race = "blood"
monster.speed = 214
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
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Eeeeek! Eeeeek!", yell = false},
	{text = "Huh! Huh! Huh!", yell = false},
	{text = "Ahhuuaaa!", yell = false},
}

monster.loot = {
	{id = 1294, chance = 30060, maxCount = 3}, -- small stone
	{id = 2148, chance = 56000, maxCount = 35}, -- gold coin
	{id = 2675, chance = 19840, maxCount = 5}, -- orange
	{id = 2676, chance = 30000, maxCount = 12}, -- banana
	{id = 2678, chance = 1960, maxCount = 3}, -- coconut
	{id = 2682, chance = 1000}, -- melon
	{id = 5883, chance = 3000}, -- ape fur
	{id = 12467, chance = 5000}, -- banana sash
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -40, target = false},
	{name = "combat", interval = 2000, chance = 35, minDamage = 0, maxDamage = -55, range = 7, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 380, duration = 5000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 25},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)