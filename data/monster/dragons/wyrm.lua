local mType = Game.createMonsterType("Wyrm")
local monster = {}

monster.description = "a wyrm"
monster.experience = 1550
monster.outfit = {
	lookType = 291,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8941
monster.health = 1825
monster.maxHealth = 1825
monster.race = "blood"
monster.speed = 200
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "GRROARR", yell = false},
	{text = "GRRR", yell = false},
}

monster.loot = {
	{id = 2148, chance = 75130, maxCount = 232}, -- gold coin
	{id = 2672, chance = 27035, maxCount = 3}, -- dragon ham
	{id = 7588, chance = 15762}, -- strong health potion
	{id = 7589, chance = 11813}, -- strong mana potion
	{id = 10582, chance = 11568}, -- wyrm scale
	{id = 2546, chance = 6179, maxCount = 10}, -- burst arrow
	{id = 2455, chance = 4643}, -- crossbow
	{id = 8871, chance = 956}, -- focus cape
	{id = 8921, chance = 788}, -- wand of draconia
	{id = 2145, chance = 710, maxCount = 3}, -- small diamond
	{id = 7889, chance = 569}, -- lightning pendant
	{id = 8920, chance = 350}, -- wand of starstorm
	{id = 8873, chance = 243}, -- hibiscus dress
	{id = 8855, chance = 80}, -- composite hornbow
	{id = 7430, chance = 72}, -- dragonbone staff
	{id = 10221, chance = 67}, -- shockwave amulet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -235, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -220, radius = 3, effect = CONST_ME_YELLOWENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -130, maxDamage = -200, effect = CONST_ME_PURPLEENERGY, target = false, length = 5, spread = 2, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -100, maxDamage = -125, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -98, maxDamage = -145, effect = CONST_ME_POFF, target = false, length = 4, spread = 0, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 35,
	armor = 34,
	{name = "combat", interval = 2000, chance = 15, minDamage = 100, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 2000, chance = 10, effect = CONST_ME_YELLOWNOTE, type = COMBAT_NONE},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 75},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)