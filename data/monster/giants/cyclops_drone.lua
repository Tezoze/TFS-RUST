local mType = Game.createMonsterType("Cyclops Drone")
local monster = {}

monster.description = "a cyclops drone"
monster.experience = 200
monster.outfit = {
	lookType = 280,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7847
monster.health = 325
monster.maxHealth = 325
monster.race = "blood"
monster.speed = 198
monster.manaCost = 525
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Fee! Fie! Foe! Fum!", yell = false},
	{text = "Luttl pest!", yell = false},
	{text = "Me makking you pulp!", yell = false},
	{text = "Humy tasy! Hum hum!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 82000, maxCount = 30}, -- gold coin
	{id = 2207, chance = 90}, -- melee ring
	{id = 2381, chance = 680}, -- halberd
	{id = 2406, chance = 8000}, -- short sword
	{id = 2490, chance = 190}, -- dark helmet
	{id = 2510, chance = 2000}, -- plate shield
	{id = 2513, chance = 1600}, -- battle shield
	{id = 2666, chance = 50430, maxCount = 2}, -- meat
	{id = 7398, chance = 120}, -- cyclops trophy
	{id = 7588, chance = 520}, -- strong health potion
	{id = 10574, chance = 6750}, -- cyclops toe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -105, target = false},
	{name = "combat", interval = 2000, chance = 35, minDamage = 0, maxDamage = -80, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)