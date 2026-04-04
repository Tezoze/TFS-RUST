local mType = Game.createMonsterType("Kongra")
local monster = {}

monster.description = "a kongra"
monster.experience = 115
monster.outfit = {
	lookType = 116,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6043
monster.health = 340
monster.maxHealth = 340
monster.race = "blood"
monster.speed = 184
monster.manaCost = 0
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
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hugah!", yell = false},
	{text = "Ungh! Ungh!", yell = false},
	{text = "Huaauaauaauaa!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 69000, maxCount = 40}, -- gold coin
	{id = 2166, chance = 300}, -- power ring
	{id = 2200, chance = 990}, -- protection amulet
	{id = 2207, chance = 230}, -- melee ring
	{id = 2463, chance = 950}, -- plate armor
	{id = 2676, chance = 30000, maxCount = 12}, -- banana
	{id = 5883, chance = 3980}, -- ape fur
	{id = 7618, chance = 570}, -- health potion
	{id = 12427, chance = 4900}, -- kongra's shoulderpad
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 18,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 260, duration = 3000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 5},
	{type = COMBAT_ICEDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)