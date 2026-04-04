local mType = Game.createMonsterType("Nightmare")
local monster = {}

monster.description = "a nightmare"
monster.experience = 1666
monster.outfit = {
	lookType = 245,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6340
monster.health = 2700
monster.maxHealth = 2700
monster.race = "blood"
monster.speed = 464
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 300,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Close your eyes... I want to show you something.", yell = false},
	{text = "I will haunt you forever!", yell = false},
	{text = "Pffffrrrrrrrrrrrr.", yell = false},
	{text = "I will make you scream.", yell = false},
	{text = "Take a ride with me.", yell = false},
	{text = "Weeeheeheeeheee!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 55}, -- gold coin
	{id = 2152, chance = 2564, maxCount = 3}, -- platinum coin
	{id = 2195, chance = 337}, -- boots of haste
	{id = 2454, chance = 95}, -- war axe
	{id = 2477, chance = 961}, -- knight legs
	{id = 2532, chance = 990}, -- ancient shield
	{id = 2547, chance = 9090, maxCount = 4}, -- power bolt
	{id = 2671, chance = 29000, maxCount = 2}, -- ham
	{id = 5669, chance = 123}, -- mysterious voodoo skull
	{id = 5944, chance = 20000}, -- soul orb
	{id = 6300, chance = 1298}, -- death ring
	{id = 6500, chance = 10000}, -- demonic essence
	{id = 6526, chance = 337}, -- skeleton decoration
	{id = 6558, chance = 19666, maxCount = 2}, -- concentrated demonic blood
	{id = 11223, chance = 15240}, -- essence of a bad dream
	{id = 11229, chance = 9090}, -- scythe leg
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -120, maxDamage = -170, range = 7, radius = 1, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -150, maxDamage = -350, range = 7, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 2000, chance = 10, minDamage = 60, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 420, duration = 5000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)