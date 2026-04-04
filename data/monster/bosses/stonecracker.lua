local mType = Game.createMonsterType("Stonecracker")
local monster = {}

monster.description = "Stonecracker"
monster.experience = 3500
monster.outfit = {
	lookType = 55,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5999
monster.health = 6500
monster.maxHealth = 6500
monster.race = "blood"
monster.speed = 280
monster.manaCost = 0
monster.maxSummons = 0

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
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "HUAHAHA!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 76225, maxCount = 100}, -- gold coin
	{id = 5893, chance = 64800}, -- perfect behemoth fang
	{id = 2666, chance = 36200}, -- meat
	{id = 5930, chance = 50500}, -- behemoth claw
	{id = 7368, chance = 11225, maxCount = 2}, -- assassin star
	{id = 2489, chance = 7650}, -- dark armor
	{id = 2416, chance = 14800}, -- crowbar
	{id = 2150, chance = 7650, maxCount = 2}, -- small amethyst
	{id = 2387, chance = 7650}, -- double axe
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 90, attack = 100, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -200, maxDamage = -280, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 40,
	{name = "speed", interval = 2000, chance = 10, effect = CONST_ME_REDSHIMMER, speed = 360, duration = 4000},
	{name = "combat", interval = 2000, chance = 10, minDamage = 500, maxDamage = 600, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 85},
	{type = COMBAT_HOLYDAMAGE, percent = 35},
	{type = COMBAT_FIREDAMAGE, percent = 40},
	{type = COMBAT_ENERGYDAMAGE, percent = 15},
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)