local mType = Game.createMonsterType("Hand of Cursed Fate")
local monster = {}

monster.description = "a hand of cursed fate"
monster.experience = 5000
monster.outfit = {
	lookType = 230,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6312
monster.health = 7500
monster.maxHealth = 7500
monster.race = "blood"
monster.speed = 260
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
	staticAttackChance = 20,
	runHealth = 3500,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2127, chance = 3500}, -- emerald bangle
	{id = 2146, chance = 11000, maxCount = 4}, -- small sapphire
	{id = 2148, chance = 60000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 60000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 67}, -- gold coin
	{id = 2152, chance = 100000, maxCount = 7}, -- platinum coin
	{id = 2153, chance = 700}, -- violet gem
	{id = 2154, chance = 5940}, -- yellow gem
	{id = 2167, chance = 3150}, -- energy ring
	{id = 2171, chance = 1005}, -- platinum amulet
	{id = 2178, chance = 9090}, -- mind stone
	{id = 2187, chance = 5590}, -- wand of inferno
	{id = 2195, chance = 540}, -- boots of haste
	{id = 2200, chance = 8740}, -- protection amulet
	{id = 2268, chance = 4200, maxCount = 8}, -- sudden death rune
	{id = 2436, chance = 700}, -- skull staff
	{id = 2476, chance = 4550}, -- knight armor
	{id = 2487, chance = 1400}, -- crown armor
	{id = 5669, chance = 247}, -- mysterious voodoo skull
	{id = 5944, chance = 31111}, -- soul orb
	{id = 6300, chance = 1750}, -- death ring
	{id = 6500, chance = 12000}, -- demonic essence
	{id = 6558, chance = 30000, maxCount = 4}, -- concentrated demonic blood
	{id = 7368, chance = 7692, maxCount = 5}, -- assassin star
	{id = 7414, chance = 495}, -- abyss hammer
	{id = 7590, chance = 19990, maxCount = 2}, -- great mana potion
	{id = 8473, chance = 18000}, -- ultimate health potion
	{id = 9971, chance = 700}, -- gold ingot
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -520, target = false, condition = {type = CONDITION_POISON, startDamage = 380, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -920, range = 1, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 10, radius = 4, effect = CONST_ME_SMALLCLOUDS, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -220, maxDamage = -880, range = 1, effect = CONST_ME_SMALLCLOUDS, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 53,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 1000, duration = 5000},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
	{name = "combat", interval = 2000, chance = 20, minDamage = 100, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)