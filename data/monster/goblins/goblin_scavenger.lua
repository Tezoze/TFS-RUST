local mType = Game.createMonsterType("Goblin Scavenger")
local monster = {}

monster.description = "a goblin scavenger"
monster.experience = 37
monster.outfit = {
	lookType = 297,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6002
monster.health = 60
monster.maxHealth = 60
monster.race = "blood"
monster.speed = 132
monster.manaCost = 310
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
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
	{text = "Shiny, shiny!", yell = false},
	{text = "Gimme gimme!", yell = false},
	{text = "You mean!", yell = false},
	{text = "All mine!", yell = false},
	{text = "Uhh!", yell = false},
}

monster.loot = {
	{id = 1294, chance = 25560, maxCount = 2}, -- small stone
	{id = 2148, chance = 50810, maxCount = 9}, -- gold coin
	{id = 2230, chance = 12450}, -- bone
	{id = 2235, chance = 7000}, -- mouldy cheese
	{id = 2379, chance = 18280}, -- dagger
	{id = 2406, chance = 8900}, -- short sword
	{id = 2449, chance = 5000}, -- bone club
	{id = 2461, chance = 10180}, -- leather helmet
	{id = 2467, chance = 7700}, -- leather armor
	{id = 2559, chance = 9790}, -- small axe
	{id = 2667, chance = 13640}, -- fish
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -15, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -30, range = 7, shootEffect = CONST_ANI_SPEAR, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -22, maxDamage = -30, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 5, minDamage = -1, maxDamage = -30, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 7,
	{name = "combat", interval = 2000, chance = 15, minDamage = 10, maxDamage = 16, effect = CONST_ME_ENERGY, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)