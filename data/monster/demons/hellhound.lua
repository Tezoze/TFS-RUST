local mType = Game.createMonsterType("Hellhound")
local monster = {}

monster.description = "a hellhound"
monster.experience = 5440
monster.outfit = {
	lookType = 240,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6332
monster.health = 7500
monster.maxHealth = 7500
monster.race = "blood"
monster.speed = 360
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "GROOOOWL!", yell = false},
}

monster.loot = {
	{id = 7426, chance = 2000}, -- amber staff
	{id = 7368, chance = 25000, maxCount = 10}, -- assassin star
	{id = 2231, chance = 900}, -- big bone
	{id = 2144, chance = 9200, maxCount = 4}, -- black pearl
	{id = 18425, chance = 12500}, -- blazing bone
	{id = 6558, chance = 20000}, -- concentrated demonic blood
	{id = 6558, chance = 20000}, -- concentrated demonic blood
	{id = 6500, chance = 20000}, -- demonic essence
	{id = 4873, chance = 400}, -- explorer brooch
	{id = 10553, chance = 10000}, -- fiery heart
	{id = 2392, chance = 7000}, -- fire sword
	{id = 2393, chance = 1000}, -- giant sword
	{id = 9971, chance = 1500}, -- gold ingot
	{id = 7590, chance = 30000, maxCount = 3}, -- great mana potion
	{id = 8472, chance = 20000}, -- great spirit potion
	{id = 2155, chance = 1000}, -- green gem
	{id = 5910, chance = 5000}, -- green piece of cloth
	{id = 2671, chance = 30000, maxCount = 6}, -- ham
	{id = 5925, chance = 10000}, -- hardened bone
	{id = 10554, chance = 20000}, -- hellhound slobber
	{id = 2430, chance = 7500}, -- knight axe
	{id = 7890, chance = 3000}, -- magma amulet
	{id = 7891, chance = 1500}, -- magma boots
	{id = 7899, chance = 800}, -- magma coat
	{id = 7894, chance = 1000}, -- magma legs
	{id = 7900, chance = 900}, -- magma monocle
	{id = 7421, chance = 1000}, -- onyx flail
	{id = 2152, chance = 100000, maxCount = 7}, -- platinum coin
	{id = 2156, chance = 4500}, -- red gem
	{id = 5911, chance = 3000}, -- red piece of cloth
	{id = 6553, chance = 1000}, -- ruthless axe
	{id = 2149, chance = 10000, maxCount = 3}, -- small emerald
	{id = 2147, chance = 10000, maxCount = 3}, -- small ruby
	{id = 9970, chance = 10000, maxCount = 3}, -- small topaz
	{id = 5944, chance = 20000}, -- soul orb
	{id = 8473, chance = 16000}, -- ultimate health potion
	{id = 2187, chance = 7000}, -- wand of inferno
	{id = 2154, chance = 4500}, -- yellow gem
	{id = 5914, chance = 6000}, -- yellow piece of cloth
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -520, target = false, condition = {type = CONDITION_POISON, startDamage = 800, interval = 2000}},
	{name = "combat", interval = 2000, chance = 5, minDamage = -300, maxDamage = -700, effect = CONST_ME_CARNIPHILA, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -395, maxDamage = -498, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -350, maxDamage = -660, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -350, maxDamage = -976, effect = CONST_ME_REDSHIMMER, target = false, length = 8, spread = 3, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -200, maxDamage = -403, radius = 1, effect = CONST_ME_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -300, maxDamage = -549, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 60,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 320, duration = 5000},
	{name = "combat", interval = 2000, chance = 20, minDamage = 220, maxDamage = 425, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)