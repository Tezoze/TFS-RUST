local mType = Game.createMonsterType("Minotaur Mage")
local monster = {}

monster.description = "a minotaur mage"
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
monster.health = 155
monster.maxHealth = 155
monster.race = "blood"
monster.speed = 170
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Learrn tha secrret uf deathhh!", yell = false},
	{text = "Kaplar!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 5000}, -- torch
	{id = 2148, chance = 85310, maxCount = 35}, -- gold coin
	{id = 2189, chance = 580}, -- wand of cosmic energy
	{id = 2461, chance = 3180}, -- leather helmet
	{id = 2649, chance = 4890}, -- leather legs
	{id = 2684, chance = 14680, maxCount = 8}, -- carrot
	{id = 5878, chance = 2010}, -- minotaur leather
	{id = 7425, chance = 990}, -- taurus mace
	{id = 7620, chance = 460}, -- mana potion
	{id = 12428, chance = 3060, maxCount = 2}, -- minotaur horn
	{id = 12429, chance = 6090}, -- purple robe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -21, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -15, maxDamage = -45, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -35, maxDamage = -95, range = 7, radius = 1, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "energyfield", interval = 2000, chance = 10, range = 7, radius = 1, shootEffect = CONST_ANI_ENERGYBALL, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 18,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)