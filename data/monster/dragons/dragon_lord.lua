local mType = Game.createMonsterType("Dragon Lord")
local monster = {}

monster.description = "a dragon lord"
monster.experience = 2100
monster.outfit = {
	lookType = 39,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5984
monster.health = 1900
monster.maxHealth = 1900
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 80,
	runHealth = 300,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "ZCHHHHHHH", yell = true},
	{text = "YOU WILL BURN!", yell = true},
}

monster.loot = {
	{id = 1976, chance = 9000}, -- book
	{id = 2033, chance = 3190}, -- golden mug
	{id = 2146, chance = 5300}, -- small sapphire
	{id = 2148, chance = 33750, maxCount = 100}, -- gold coin
	{id = 2148, chance = 33750, maxCount = 100}, -- gold coin
	{id = 2148, chance = 33750, maxCount = 45}, -- gold coin
	{id = 2167, chance = 5250}, -- energy ring
	{id = 2177, chance = 680}, -- life crystal
	{id = 2392, chance = 290}, -- fire sword
	{id = 2479, chance = 360}, -- strange helmet
	{id = 2492, chance = 170}, -- dragon scale mail
	{id = 2498, chance = 280}, -- royal helmet
	{id = 2528, chance = 250}, -- tower shield
	{id = 2547, chance = 6700, maxCount = 7}, -- power bolt
	{id = 2672, chance = 80000, maxCount = 5}, -- dragon ham
	{id = 2796, chance = 12000}, -- green mushroom
	{id = 5882, chance = 3920}, -- red dragon scale
	{id = 5948, chance = 3040}, -- red dragon leather
	{id = 7378, chance = 8800, maxCount = 3}, -- royal spear
	{id = 7399, chance = 80}, -- dragon lord trophy
	{id = 7402, chance = 100}, -- dragon slayer
	{id = 7588, chance = 970}, -- strong health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -230, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -100, maxDamage = -200, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 2000, chance = 10, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 2000, chance = 15, minDamage = -150, maxDamage = -230, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 34,
	armor = 34,
	{name = "combat", interval = 2000, chance = 15, minDamage = 57, maxDamage = 93, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 80},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)