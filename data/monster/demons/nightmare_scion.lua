local mType = Game.createMonsterType("Nightmare Scion")
local monster = {}

monster.description = "a nightmare scion"
monster.experience = 1350
monster.outfit = {
	lookType = 321,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9919
monster.health = 1400
monster.maxHealth = 1400
monster.race = "blood"
monster.speed = 440
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
	{text = "Weeeheeheee!", yell = false},
	{text = "Pffffrrrrrrrrrrrr.", yell = false},
	{text = "Peak a boo, I killed you!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 53}, -- gold coin
	{id = 2152, chance = 961, maxCount = 3}, -- platinum coin
	{id = 2491, chance = 666}, -- crown helmet
	{id = 2666, chance = 50000, maxCount = 4}, -- meat
	{id = 6300, chance = 250}, -- death ring
	{id = 6574, chance = 280}, -- bar of chocolate
	{id = 7387, chance = 340}, -- diamond sceptre
	{id = 7451, chance = 270}, -- shadow sceptre
	{id = 8871, chance = 340}, -- focus cape
	{id = 9941, chance = 100}, -- crystal of focus
	{id = 11223, chance = 7692}, -- essence of a bad dream
	{id = 11229, chance = 4761}, -- scythe leg
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -140, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -115, maxDamage = -180, range = 7, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -70, maxDamage = -130, range = 7, radius = 1, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 25,
	{name = "combat", interval = 2000, chance = 5, minDamage = 60, maxDamage = 95, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)