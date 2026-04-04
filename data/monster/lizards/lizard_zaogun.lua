local mType = Game.createMonsterType("Lizard Zaogun")
local monster = {}

monster.description = "a lizard zaogun"
monster.experience = 1700
monster.outfit = {
	lookType = 343,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11284
monster.health = 2955
monster.maxHealth = 2955
monster.race = "blood"
monster.speed = 276
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hissss!", yell = false},
	{text = "Cowardzz!", yell = false},
	{text = "Softzzkinzz from zze zzouzz!", yell = false},
	{text = "Zztand and fight!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 31500, maxCount = 100}, -- gold coin
	{id = 2148, chance = 31500, maxCount = 100}, -- gold coin
	{id = 2148, chance = 31000, maxCount = 68}, -- gold coin
	{id = 2149, chance = 4830, maxCount = 5}, -- small emerald
	{id = 2152, chance = 48900, maxCount = 2}, -- platinum coin
	{id = 2528, chance = 1000}, -- tower shield
	{id = 5876, chance = 14360}, -- lizard leather
	{id = 5881, chance = 12520}, -- lizard scale
	{id = 7588, chance = 1900}, -- strong health potion
	{id = 7591, chance = 7000, maxCount = 3}, -- great health potion
	{id = 11206, chance = 2170}, -- red lantern
	{id = 11301, chance = 530}, -- Zaoan armor
	{id = 11303, chance = 1000}, -- Zaoan shoes
	{id = 11304, chance = 1001}, -- Zaoan legs
	{id = 11330, chance = 8280}, -- zaogun flag
	{id = 11331, chance = 14980}, -- zaogun shoulderplates
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -349, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -220, maxDamage = -375, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 42,
	{name = "combat", interval = 2000, chance = 10, minDamage = 175, maxDamage = 275, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 45},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 15},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)