local mType = Game.createMonsterType("Elder Wyrm")
local monster = {}

monster.description = "an elder wyrm"
monster.experience = 2500
monster.outfit = {
	lookType = 561,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 21283
monster.health = 2700
monster.maxHealth = 2700
monster.race = "blood"
monster.speed = 260
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
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
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 250,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "GRRR!", yell = false},
	{text = "GROOOOAAAAAAAAR!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 180}, -- gold coin
	{id = 2672, chance = 34520, maxCount = 3}, -- dragon ham
	{id = 2152, chance = 24680, maxCount = 3}, -- platinum coin
	{id = 7589, chance = 20290}, -- strong mana potion
	{id = 7588, chance = 19840}, -- strong health potion
	{id = 10582, chance = 16680}, -- wyrm scale
	{id = 2455, chance = 8150}, -- crossbow
	{id = 5944, chance = 4830}, -- soul orb
	{id = 2145, chance = 4700, maxCount = 5}, -- small diamond
	{id = 8921, chance = 1530}, -- wand of draconia
	{id = 2547, chance = 850, maxCount = 10}, -- power bolt
	{id = 7889, chance = 820}, -- lightning pendant
	{id = 8918, chance = 670}, -- lightning legs
	{id = 8920, chance = 620}, -- wand of starstorm
	{id = 7885, chance = 230}, -- lightning boots
	{id = 8855, chance = 210}, -- composite hornbow
	{id = 7430, chance = 200}, -- dragonbone staff
	{id = 7451, chance = 200}, -- shadow sceptre
	{id = 8869, chance = 170}, -- lightning robe
	{id = 10221, chance = 110}, -- shockwave amulet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -360, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -90, maxDamage = -150, radius = 4, effect = CONST_ME_TELEPORT, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -140, maxDamage = -250, radius = 5, effect = CONST_ME_PURPLEENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -180, effect = CONST_ME_YELLOWENERGY, target = false, length = 8, spread = 0, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -200, maxDamage = -300, effect = CONST_ME_BLACKSMOKE, target = true, length = 5, spread = 2, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 45,
	{name = "combat", interval = 2000, chance = 15, minDamage = 100, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 80},
	{type = COMBAT_FIREDAMAGE, percent = 25},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)