local mType = Game.createMonsterType("Grand Mother Foulscale")
local monster = {}

monster.description = "Grand Mother Foulscale"
monster.experience = 1400
monster.outfit = {
	lookType = 34,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5973
monster.health = 1850
monster.maxHealth = 1850
monster.race = "blood"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 400,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "GROOAAARRR!", yell = true},
	{text = "FCHHHHH!", yell = true},
}

monster.loot = {
	{id = 2145, chance = 500}, -- small diamond
	{id = 2148, chance = 37500, maxCount = 70}, -- gold coin
	{id = 2148, chance = 37500, maxCount = 50}, -- gold coin
	{id = 2177, chance = 150}, -- life crystal
	{id = 2187, chance = 1800}, -- wand of inferno
	{id = 2387, chance = 1333}, -- double axe
	{id = 2397, chance = 5000}, -- longsword
	{id = 2398, chance = 21500}, -- mace
	{id = 2406, chance = 25000}, -- short sword
	{id = 2409, chance = 500}, -- serpent sword
	{id = 2413, chance = 2000}, -- broadsword
	{id = 2434, chance = 600}, -- dragon hammer
	{id = 2455, chance = 10000}, -- crossbow
	{id = 2457, chance = 3000}, -- steel helmet
	{id = 2509, chance = 14000}, -- steel shield
	{id = 2516, chance = 500}, -- dragon shield
	{id = 2546, chance = 4000, maxCount = 12}, -- burst arrow
	{id = 2647, chance = 2000}, -- plate legs
	{id = 2672, chance = 15500, maxCount = 3}, -- dragon ham
	{id = 5877, chance = 100000}, -- green dragon leather
	{id = 5920, chance = 100000}, -- green dragon scale
	{id = 7430, chance = 650}, -- dragonbone staff
	{id = 7588, chance = 1750}, -- strong health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = -20, maxDamage = -170, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -45, maxDamage = -85, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 8, minDamage = -90, maxDamage = -150, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 27,
	{name = "combat", interval = 1000, chance = 17, minDamage = 34, maxDamage = 66, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 80},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Dragon Hatchling", chance = 40, interval = 4000, max = 4},
}

mType:register(monster)