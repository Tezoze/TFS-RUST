local mType = Game.createMonsterType("Assassin")
local monster = {}

monster.description = "an assassin"
monster.experience = 105
monster.outfit = {
	lookType = 152,
	lookHead = 114,
	lookBody = 95,
	lookLegs = 95,
	lookFeet = 95,
	lookAddons = 3,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 175
monster.maxHealth = 175
monster.race = "blood"
monster.speed = 224
monster.manaCost = 450
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Die!", yell = false},
	{text = "Feel the hand of death!", yell = false},
	{text = "You are on my deathlist!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 29980, maxCount = 2}, -- torch
	{id = 2145, chance = 220}, -- small diamond
	{id = 2148, chance = 83210, maxCount = 50}, -- gold coin
	{id = 2148, chance = 7250, maxCount = 14}, -- gold coin
	{id = 2403, chance = 9500}, -- knife
	{id = 2404, chance = 4000}, -- combat knife
	{id = 2457, chance = 3230}, -- steel helmet
	{id = 2509, chance = 970}, -- steel shield
	{id = 2510, chance = 1900}, -- plate shield
	{id = 2513, chance = 1600}, -- battle shield
	{id = 3968, chance = 480}, -- leopard armor
	{id = 3969, chance = 230}, -- horseman helmet
	{id = 7366, chance = 4200, maxCount = 7}, -- viper star
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -120, interval = 2000, target = false},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = 0, maxDamage = -40, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_THROWINGSTAR},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -120, maxDamage = -160, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 17,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)