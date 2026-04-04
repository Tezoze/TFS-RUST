local mType = Game.createMonsterType("Slick Water Elemental")
local monster = {}

monster.description = "a slick water elemental"
monster.experience = 450
monster.outfit = {
	lookType = 286,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8965
monster.health = 550
monster.maxHealth = 550
monster.race = "undead"
monster.speed = 280
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
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "BLUUUUB", yell = false},
	{text = "SPLISH SPLASH", yell = false},
}

monster.loot = {
	{id = 2148, chance = 22500, maxCount = 70}, -- gold coin
	{id = 2148, chance = 22500, maxCount = 60}, -- gold coin
	{id = 7839, chance = 2575, maxCount = 3}, -- shiver arrow
	{id = 8302, chance = 6000}, -- iced soil
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -175, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -130, range = 7, shootEffect = CONST_ANI_EARTH, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 10, minDamage = 0, maxDamage = -220, range = 6, shootEffect = CONST_ANI_SNOWBALL, target = true, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 18, minDamage = 0, maxDamage = -103, range = 4, shootEffect = CONST_ANI_SMALLICE, target = true, type = COMBAT_ICEDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 15, minDamage = 90, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 40},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)