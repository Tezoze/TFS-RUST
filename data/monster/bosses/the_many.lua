local mType = Game.createMonsterType("The Many")
local monster = {}

monster.description = "The Many"
monster.experience = 4000
monster.outfit = {
	lookType = 121,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6048
monster.health = 5000
monster.maxHealth = 5000
monster.race = "blood"
monster.speed = 260
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
	runHealth = 300,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2197, chance = 90000}, -- stone skin amulet
	{id = 7589, chance = 83000, maxCount = 5}, -- strong mana potion
	{id = 10219, chance = 80000}, -- sacred tree amulet
	{id = 2475, chance = 79000}, -- warrior helmet
	{id = 2146, chance = 77000, maxCount = 5}, -- small sapphire
	{id = 9971, chance = 60000, maxCount = 3}, -- gold ingot
	{id = 2536, chance = 53000}, -- medusa shield
	{id = 10523, chance = 37000}, -- egg of the many
	{id = 2498, chance = 20000}, -- royal helmet
	{id = 2476, chance = 10000}, -- knight armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -270, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -65, maxDamage = -320, effect = CONST_ME_CARNIPHILA, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 2000, chance = 25, range = 7, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENBUBBLE, target = true, speed = -300, duration = 15000},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -250, effect = CONST_ME_BLUEBUBBLE, target = false, length = 8, spread = 3, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -70, maxDamage = -155, range = 7, shootEffect = CONST_ANI_ICE, effect = CONST_ME_ICEATTACK, target = true, type = COMBAT_ICEDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 35,
	{name = "combat", interval = 2000, chance = 20, minDamage = 260, maxDamage = 407, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)