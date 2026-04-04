local mType = Game.createMonsterType("Overcharged Energy Elemental")
local monster = {}

monster.description = "an overcharged energy elemental"
monster.experience = 1300
monster.outfit = {
	lookType = 290,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8966
monster.health = 1200
monster.maxHealth = 1200
monster.race = "undead"
monster.speed = 300
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 20000,
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
	staticAttackChance = 85,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "BZZZZZZZZZZ", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 56}, -- gold coin
	{id = 2150, chance = 10000, maxCount = 2}, -- small amethyst
	{id = 7439, chance = 2173}, -- berserk potion
	{id = 7591, chance = 10000}, -- great health potion
	{id = 8303, chance = 14285}, -- energy soil
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false},
	{name = "combat", interval = 1000, chance = 11, minDamage = 0, maxDamage = -250, radius = 4, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_PURPLEENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 12, minDamage = 0, maxDamage = -300, range = 3, effect = CONST_ME_PURPLEENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 12, minDamage = 0, maxDamage = -200, radius = 4, effect = CONST_ME_POFF, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 35,
	{name = "combat", interval = 2000, chance = 15, minDamage = 90, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)