local mType = Game.createMonsterType("Fury")
local monster = {}

monster.description = "a fury"
monster.experience = 3600
monster.outfit = {
	lookType = 149,
	lookHead = 94,
	lookBody = 77,
	lookLegs = 96,
	lookFeet = 0,
	lookAddons = 3,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 4100
monster.maxHealth = 4100
monster.race = "blood"
monster.speed = 400
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
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ahhhhrrrr!", yell = false},
	{text = "Waaaaah!", yell = false},
	{text = "Caaarnaaage!", yell = false},
	{text = "Dieee!", yell = false},
}

monster.loot = {
	{id = 2124, chance = 410}, -- crystal ring
	{id = 2148, chance = 30000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 30000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 38000, maxCount = 69}, -- gold coin
	{id = 2152, chance = 2800, maxCount = 4}, -- platinum coin
	{id = 2181, chance = 20000}, -- terra rod
	{id = 2470, chance = 130}, -- golden legs
	{id = 2645, chance = 790}, -- steel boots
	{id = 5022, chance = 1500, maxCount = 4}, -- orichalcum pearl
	{id = 5911, chance = 4000}, -- red piece of cloth
	{id = 5944, chance = 21500}, -- soul orb
	{id = 5944, chance = 50}, -- soul orb
	{id = 6301, chance = 60}, -- death ring
	{id = 6500, chance = 22500}, -- demonic essence
	{id = 6558, chance = 35000, maxCount = 3}, -- concentrated demonic blood
	{id = 7404, chance = 660}, -- assassin dagger
	{id = 7456, chance = 2000}, -- noble axe
	{id = 7591, chance = 10500}, -- great health potion
	{id = 8844, chance = 29280, maxCount = 4}, -- jalapeno pepper
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -510, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -200, maxDamage = -300, effect = CONST_ME_EXPLOSIONAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -120, maxDamage = -700, effect = CONST_ME_REDSPARK, target = false, length = 8, spread = 0, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -120, maxDamage = -300, radius = 4, effect = CONST_ME_REDSPARK, target = false, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -120, maxDamage = -300, radius = 3, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -125, maxDamage = -250, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "speed", interval = 2000, chance = 15, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, speed = -800, duration = 30000},
}

monster.defenses = {
	defense = 20,
	armor = 35,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 800, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)