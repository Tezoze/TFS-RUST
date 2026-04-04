local mType = Game.createMonsterType("Lost Soul")
local monster = {}

monster.description = "a lost soul"
monster.experience = 4000
monster.outfit = {
	lookType = 232,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6310
monster.health = 5800
monster.maxHealth = 5800
monster.race = "undead"
monster.speed = 380
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 15
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
	runHealth = 450,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Forgive Meee!", yell = false},
	{text = "Mouuuurn meeee!", yell = false},
	{text = "Help meee!", yell = false},
}

monster.loot = {
	{id = 2133, chance = 1500}, -- ruby necklace
	{id = 2143, chance = 10000, maxCount = 3}, -- white pearl
	{id = 2144, chance = 12000, maxCount = 3}, -- black pearl
	{id = 2148, chance = 100000, maxCount = 198}, -- gold coin
	{id = 2152, chance = 100000, maxCount = 7}, -- platinum coin
	{id = 2156, chance = 15000}, -- red gem
	{id = 2197, chance = 2780}, -- stone skin amulet
	{id = 2260, chance = 35250, maxCount = 3}, -- blank rune
	{id = 2436, chance = 850}, -- skull staff
	{id = 2528, chance = 740}, -- tower shield
	{id = 5741, chance = 170}, -- skull helmet
	{id = 5806, chance = 4950}, -- silver goblet
	{id = 5944, chance = 15000}, -- soul orb
	{id = 6300, chance = 2170}, -- death ring
	{id = 6500, chance = 7500}, -- demonic essence
	{id = 6526, chance = 1250}, -- skeleton decoration
	{id = 7407, chance = 740}, -- haunted blade
	{id = 7413, chance = 1000}, -- titan axe
	{id = 7590, chance = 14200, maxCount = 2}, -- great mana potion
	{id = 7591, chance = 8800, maxCount = 2}, -- great health potion
	{id = 9810, chance = 3500},
	{id = 11233, chance = 33010}, -- unholy bone
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -420, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -40, maxDamage = -210, effect = CONST_ME_REDSHIMMER, target = false, length = 3, spread = 0, type = COMBAT_DEATHDAMAGE},
	{name = "speed", interval = 2000, chance = 20, radius = 6, effect = CONST_ME_SMALLCLOUDS, target = false, speed = -800, duration = 4000},
}

monster.defenses = {
	defense = 30,
	armor = 28,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)