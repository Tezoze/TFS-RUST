local mType = Game.createMonsterType("Banshee")
local monster = {}

monster.description = "a banshee"
monster.experience = 900
monster.outfit = {
	lookType = 78,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6019
monster.health = 1000
monster.maxHealth = 1000
monster.race = "undead"
monster.speed = 220
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
	runHealth = 500,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Dance for me your dance of death!", yell = false},
	{text = "Let the music play!", yell = false},
	{text = "I will mourn your death!", yell = false},
	{text = "Are you ready to rock?", yell = false},
	{text = "Feel my gentle kiss of death.", yell = false},
	{text = "That's what I call easy listening!", yell = false},
	{text = "IIIIEEEeeeeeehhhHHHH!", yell = true},
}

monster.loot = {
	{id = 2047, chance = 70000}, -- candlestick
	{id = 2121, chance = 460}, -- wedding ring
	{id = 2124, chance = 60}, -- crystal ring
	{id = 2134, chance = 1250}, -- silver brooch
	{id = 2143, chance = 1010}, -- white pearl
	{id = 2144, chance = 2030}, -- black pearl
	{id = 2148, chance = 30000, maxCount = 80}, -- gold coin
	{id = 2170, chance = 8700}, -- silver amulet
	{id = 2175, chance = 520}, -- spellbook
	{id = 2177, chance = 70}, -- life crystal
	{id = 2197, chance = 820}, -- stone skin amulet
	{id = 2214, chance = 730}, -- ring of healing
	{id = 2372, chance = 910}, -- lyre
	{id = 2411, chance = 1350}, -- poison dagger
	{id = 2655, chance = 150}, -- red robe
	{id = 2656, chance = 700}, -- blue robe
	{id = 2657, chance = 6050}, -- simple dress
	{id = 7589, chance = 680}, -- strong mana potion
	{id = 7884, chance = 340}, -- terra mantle
	{id = 11337, chance = 4150}, -- petrified scream
	{id = 12402, chance = 4810}, -- hair of a banshee
	{id = 13307, chance = 30}, -- sweet smelling bait
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false, condition = {type = CONDITION_POISON, startDamage = 3, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -200, radius = 4, effect = CONST_ME_REDNOTE, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 5, minDamage = -55, maxDamage = -350, range = 1, radius = 1, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "speed", interval = 2000, chance = 10, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -300, duration = 15000},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 2000, chance = 15, minDamage = 120, maxDamage = 190, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)