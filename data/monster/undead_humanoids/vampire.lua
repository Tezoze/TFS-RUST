local mType = Game.createMonsterType("Vampire")
local monster = {}

monster.description = "a vampire"
monster.experience = 305
monster.outfit = {
	lookType = 68,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6006
monster.health = 475
monster.maxHealth = 475
monster.race = "blood"
monster.speed = 238
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
	runHealth = 30,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "BLOOD!", yell = true},
	{text = "Let me kiss your neck", yell = false},
	{text = "I smell warm blood!", yell = false},
	{text = "I call you, my bats! Come!", yell = false},
}

monster.loot = {
	{id = 2127, chance = 230}, -- emerald bangle
	{id = 2144, chance = 1800}, -- black pearl
	{id = 2148, chance = 90230, maxCount = 60}, -- gold coin
	{id = 2172, chance = 220}, -- bronze amulet
	{id = 2229, chance = 1000}, -- skull
	{id = 2383, chance = 1000}, -- spike sword
	{id = 2396, chance = 420}, -- ice rapier
	{id = 2412, chance = 1560}, -- katana
	{id = 2479, chance = 420}, -- strange helmet
	{id = 2534, chance = 230}, -- vampire shield
	{id = 2747, chance = 1910}, -- grave flower
	{id = 7588, chance = 1500}, -- strong health potion
	{id = 10602, chance = 7600}, -- vampire teeth
	{id = 12405, chance = 5100}, -- blood preservation
	{id = 5905, chance = 5100}, -- Vampire Dust
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -50, maxDamage = -200, range = 1, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 15, range = 1, effect = CONST_ME_REDSHIMMER, target = true, speed = -400, duration = 60000},
}

monster.defenses = {
	defense = 30,
	armor = 28,
	{name = "outfit", interval = 4000, chance = 10, effect = CONST_ME_GROUNDSHAKER, monster = "bat", duration = 5000},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 3000},
	{name = "combat", interval = 2000, chance = 15, minDamage = 15, maxDamage = 25, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 35},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)