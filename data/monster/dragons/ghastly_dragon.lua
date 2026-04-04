local mType = Game.createMonsterType("Ghastly Dragon")
local monster = {}

monster.description = "a ghastly dragon"
monster.experience = 4600
monster.outfit = {
	lookType = 351,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11362
monster.health = 7800
monster.maxHealth = 7800
monster.race = "undead"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 5
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
	runHealth = 366,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "EMBRACE MY GIFTS!", yell = true},
	{text = "I WILL FEAST ON YOUR SOUL!", yell = true},
}

monster.loot = {
	{id = 2148, chance = 33725, maxCount = 100}, -- gold coin
	{id = 2148, chance = 33725, maxCount = 100}, -- gold coin
	{id = 2148, chance = 33725, maxCount = 66}, -- gold coin
	{id = 2152, chance = 29840, maxCount = 2}, -- platinum coin
	{id = 5944, chance = 12170}, -- soul orb
	{id = 6500, chance = 8920}, -- demonic essence
	{id = 7590, chance = 30560, maxCount = 2}, -- great mana potion
	{id = 7885, chance = 3130}, -- terra legs
	{id = 7886, chance = 9510}, -- terra boots
	{id = 8472, chance = 29460, maxCount = 2}, -- great spirit potion
	{id = 8473, chance = 24700}, -- ultimate health potion
	{id = 9810, chance = 180},
	{id = 11227, chance = 860}, -- shiny stone
	{id = 11240, chance = 200}, -- guardian boots
	{id = 11301, chance = 870}, -- Zaoan armor
	{id = 11302, chance = 150}, -- Zaoan helmet
	{id = 11303, chance = 870}, -- Zaoan shoes
	{id = 11304, chance = 1400}, -- Zaoan legs
	{id = 11305, chance = 1470}, -- drakinata
	{id = 11307, chance = 100}, -- Zaoan sword
	{id = 11309, chance = 15100}, -- twin hooks
	{id = 11323, chance = 15020}, -- Zaoan halberd
	{id = 11355, chance = 690}, -- spellweaver's robe
	{id = 11366, chance = 6650}, -- ghastly dragon head
	{id = 11367, chance = 19830}, -- undead heart
	{id = 11368, chance = 810}, -- jade hat
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -603, interval = 2000, target = false},
	{name = "ghastly dragon curse", interval = 2000, chance = 5, range = 5, target = true},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -920, maxDamage = -1280, range = 5, effect = CONST_ME_SMALLCLOUDS, target = true},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -80, maxDamage = -230, interval = 2000, chance = 15, range = 7, target = true, effect = CONST_ME_REDSHIMMER},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = -120, maxDamage = -250, interval = 2000, chance = 10, length = 8, spread = 3, target = false, effect = CONST_ME_BLUEBUBBLE},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = -110, maxDamage = -180, interval = 2000, chance = 15, radius = 4, target = false, effect = CONST_ME_MORTAREA},
	{name = "speed", interval = 2000, chance = 20, range = 7, target = true, effect = CONST_ME_SMALLCLOUDS, speed = -800, duration = 30000},
}

monster.defenses = {
	defense = 35,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -15},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)