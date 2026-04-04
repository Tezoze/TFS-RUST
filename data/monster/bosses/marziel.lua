local mType = Game.createMonsterType("Marziel")
local monster = {}

monster.description = "Marziel"
monster.experience = 3000
monster.outfit = {
	lookType = 287,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8937
monster.health = 1900
monster.maxHealth = 1900
monster.race = "undead"
monster.speed = 270
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	ignorespawnblock = false,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "My brides will show you how to suffer beautifully.", yell = false},
	{text = "I smell warm blood.", yell = false},
	{text = "Why are you disturbing me and my brides?", yell = false},
	{text = "You shouldn't have come.", yell = false},
	{text = "You! You had no right to read my diary!", yell = false},
	{text = "I... want... your... blood!", yell = false},
}

monster.loot = {
	{id = 12405, chance = 88890}, -- blood preservation
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 9280, maxCount = 5}, -- platinum coin
	{id = 2534, chance = 610}, -- vampire shield
	{id = 7419, chance = 300}, -- dreaded cleaver
	{id = 7588, chance = 22530}, -- strong health potion
	{id = 2214, chance = 12020}, -- ring of healing
	{id = 2144, chance = 1830}, -- black pearl
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 90, attack = 50, target = false},
	{name = "combat", interval = 2000, chance = 24, minDamage = 0, maxDamage = -600, range = 7, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_REDSPARK, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 12, minDamage = 40, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
	{name = "outfit", interval = 2000, chance = 6, effect = CONST_ME_BLUESHIMMER, monster = "bat", duration = 6000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 43},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 100},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_LIFEDRAINDAMAGE, percent = 0},
	{type = COMBAT_MANADRAINDAMAGE, percent = 0},
	{type = COMBAT_DROWNDAMAGE, percent = 0},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = 100},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)