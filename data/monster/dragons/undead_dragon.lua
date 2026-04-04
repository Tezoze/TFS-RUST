local mType = Game.createMonsterType("Undead Dragon")
local monster = {}

monster.description = "an undead dragon"
monster.experience = 7200
monster.outfit = {
	lookType = 231,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6306
monster.health = 8350
monster.maxHealth = 8350
monster.race = "undead"
monster.speed = 330
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
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "FEEEED MY ETERNAL HUNGER!", yell = true},
	{text = "I SENSE LIFE", yell = true},
}

monster.loot = {
	{id = 2033, chance = 6002}, -- golden mug
	{id = 2144, chance = 22780, maxCount = 2}, -- black pearl
	{id = 2146, chance = 28370, maxCount = 2}, -- small sapphire
	{id = 2148, chance = 35500, maxCount = 100}, -- gold coin
	{id = 2148, chance = 55500, maxCount = 98}, -- gold coin
	{id = 2152, chance = 52000, maxCount = 5}, -- platinum coin
	{id = 2177, chance = 2500}, -- life crystal
	{id = 2454, chance = 1290}, -- war axe
	{id = 2466, chance = 860}, -- golden armor
	{id = 2476, chance = 5500}, -- knight armor
	{id = 2498, chance = 1720}, -- royal helmet
	{id = 2547, chance = 15190, maxCount = 15}, -- power bolt
	{id = 5925, chance = 14180}, -- hardened bone
	{id = 6300, chance = 1150}, -- death ring
	{id = 6500, chance = 12460}, -- demonic essence
	{id = 7368, chance = 26650, maxCount = 5}, -- assassin star
	{id = 7402, chance = 860}, -- dragon slayer
	{id = 7430, chance = 4000}, -- dragonbone staff
	{id = 7590, chance = 21490}, -- great mana potion
	{id = 7591, chance = 21200}, -- great health potion
	{id = 8885, chance = 430}, -- divine plate
	{id = 8889, chance = 290}, -- skullcracker armor
	{id = 9971, chance = 570}, -- gold ingot
	{id = 11233, chance = 33380}, -- unholy bone
	{id = 11355, chance = 860}, -- spellweaver's robe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -480, target = false},
	{name = "combat", interval = 2000, chance = 5, minDamage = -300, maxDamage = -400, range = 7, radius = 4, effect = CONST_ME_REDSPARK, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -125, maxDamage = -600, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -100, maxDamage = -390, range = 7, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -180, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -150, maxDamage = -690, effect = CONST_ME_POISON, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -300, maxDamage = -700, effect = CONST_ME_REDSHIMMER, target = false, length = 8, spread = 3, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -200, radius = 3, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 40,
	{name = "combat", interval = 2000, chance = 15, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)