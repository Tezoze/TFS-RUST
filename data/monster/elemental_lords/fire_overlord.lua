local mType = Game.createMonsterType("Fire Overlord")
local monster = {}

monster.description = "a Fire Overlord"
monster.experience = 2800
monster.outfit = {
	lookType = 243,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8964
monster.health = 4000
monster.maxHealth = 4000
monster.race = "fire"
monster.speed = 300
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 75}, -- gold coin
	{id = 2152, chance = 50000, maxCount = 3}, -- platinum coin
	{id = 7899, chance = 819}, -- magma coat
	{id = 8304, chance = 100000}, -- eternal flames
	{id = 10553, chance = 100000}, -- fiery heart
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -450, target = false, condition = {type = CONDITION_FIRE, startDamage = 650, interval = 2000}},
	{name = "firefield", interval = 2000, chance = 15, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 1000, chance = 15, minDamage = -300, maxDamage = -900, effect = CONST_ME_FIREATTACK, target = false, length = 1, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 13, minDamage = -200, maxDamage = -350, radius = 4, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 1},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 80},
	{type = COMBAT_ICEDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)