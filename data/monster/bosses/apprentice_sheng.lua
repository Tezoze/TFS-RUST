local mType = Game.createMonsterType("Apprentice Sheng")
local monster = {}

monster.description = "Apprentice Sheng"
monster.experience = 150
monster.outfit = {
	lookType = 23,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5981
monster.health = 95
monster.maxHealth = 95
monster.race = "blood"
monster.speed = 170
monster.manaCost = 0
monster.maxSummons = 2

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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 20,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I will protect the secrets of my master!", yell = false},
	{text = "This isle will become ours alone", yell = false},
	{text = "Kaplar!", yell = false},
	{text = "You already know too much.", yell = false},
}

monster.loot = {
	{id = 5878, chance = 100000}, -- minotaur leather
	{id = 2162, chance = 80000}, -- magic light wand
	{id = 2148, chance = 30000, maxCount = 10}, -- gold coin
	{id = 2050, chance = 30000, maxCount = 2}, -- torch
	{id = 2649, chance = 20000}, -- leather legs
	{id = 2403, chance = 10000}, -- knife
	{id = 2461, chance = 10000}, -- leather helmet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -10, target = false},
	{name = "energyfield", interval = 1000, chance = 8, range = 7, radius = 1, shootEffect = CONST_ANI_ENERGY, target = true},
	{name = "combat", interval = 1000, chance = 14, minDamage = 0, maxDamage = -25, range = 7, shootEffect = CONST_ANI_ENERGYBALL, effect = CONST_ME_ENERGYAREA, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -45, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 13,
	armor = 12,
	{name = "combat", interval = 4000, chance = 15, minDamage = 10, maxDamage = 20, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Hyaena", chance = 30, interval = 5000, max = 2},
}

mType:register(monster)