local mType = Game.createMonsterType("Ice Witch")
local monster = {}

monster.description = "an ice witch"
monster.experience = 580
monster.outfit = {
	lookType = 149,
	lookHead = 0,
	lookBody = 47,
	lookLegs = 105,
	lookFeet = 105,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 650
monster.maxHealth = 650
monster.race = "blood"
monster.speed = 228
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 70,
	targetDistance = 4,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "So you think you are cool?", yell = false},
	{text = "I hope it is not too cold for you! HeHeHe.", yell = false},
	{text = "Freeze!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 29630, maxCount = 90}, -- gold coin
	{id = 2423, chance = 920}, -- clerical mace
	{id = 2663, chance = 430}, -- mystic turban
	{id = 2796, chance = 1310}, -- green mushroom
	{id = 7290, chance = 5300}, -- shard
	{id = 7387, chance = 330}, -- diamond sceptre
	{id = 7441, chance = 10000}, -- ice cube
	{id = 7449, chance = 400}, -- crystal sword
	{id = 7459, chance = 90}, -- pair of earmuffs
	{id = 7589, chance = 820}, -- strong mana potion
	{id = 7892, chance = 280}, -- glacier shoes
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
	{name = "outfit", interval = 2000, chance = 1, range = 7, shootEffect = CONST_ANI_SNOWBALL, effect = CONST_ME_BLUESHIMMER, target = true},
	{name = "combat", interval = 2000, chance = 10, minDamage = -60, maxDamage = -130, effect = CONST_ME_ICETORNADO, target = false, length = 5, spread = 2, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -55, maxDamage = -115, range = 7, shootEffect = CONST_ANI_SMALLICE, effect = CONST_ME_ICEATTACK, target = true, type = COMBAT_ICEDAMAGE},
	{name = "speed", interval = 2000, chance = 15, range = 7, shootEffect = CONST_ANI_SMALLICE, effect = CONST_ME_ICETORNADO, target = true, speed = -600, duration = 20000},
}

monster.defenses = {
	defense = 20,
	armor = 70,
	{name = "combat", interval = 2000, chance = 25, minDamage = 90, maxDamage = 120, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 50},
	{type = COMBAT_EARTHDAMAGE, percent = 40},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)