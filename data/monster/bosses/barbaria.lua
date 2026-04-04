local mType = Game.createMonsterType("Barbaria")
local monster = {}

monster.description = "Barbaria"
monster.experience = 355
monster.outfit = {
	lookType = 264,
	lookHead = 78,
	lookBody = 116,
	lookLegs = 95,
	lookFeet = 121,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 345
monster.maxHealth = 345
monster.race = "blood"
monster.speed = 280
monster.manaCost = 0
monster.maxSummons = 1

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 90,
	targetDistance = 4,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "To me, creatures of the wild!", yell = false},
	{text = "My instincts tell me about your cowardice.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 48000, maxCount = 35}, -- gold coin
	{id = 2464, chance = 11000}, -- chain armor
	{id = 3965, chance = 12500}, -- hunting spear
	{id = 7343, chance = 1000}, -- fur bag
	{id = 2050, chance = 25000}, -- torch
	{id = 1958, chance = 15000},
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 60, attack = 20, target = false},
	{name = "combat", interval = 2000, chance = 34, minDamage = -30, maxDamage = -80, range = 7, radius = 1, shootEffect = CONST_ANI_SNOWBALL, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 3000, chance = 20, minDamage = -35, maxDamage = -70, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 10,
	{name = "combat", interval = 1000, chance = 25, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "War Wolf", chance = 40, interval = 2000, max = 1},
}

mType:register(monster)