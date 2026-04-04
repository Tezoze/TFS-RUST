local mType = Game.createMonsterType("Serpent Spawn")
local monster = {}

monster.description = "a serpent spawn"
monster.experience = 3050
monster.outfit = {
	lookType = 220,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6061
monster.health = 3000
monster.maxHealth = 3000
monster.race = "venom"
monster.speed = 234
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
	staticAttackChance = 80,
	runHealth = 275,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Sssssouls for the one", yell = false},
	{text = "HISSSS", yell = true},
	{text = "Tsssse one will risssse again", yell = false},
	{text = "I bring you deathhhh, mortalssss", yell = false},
}

monster.loot = {
	{id = 2148, chance = 97250, maxCount = 245}, -- gold coin
	{id = 2796, chance = 18290}, -- green mushroom
	{id = 10611, chance = 15010}, -- snake skin
	{id = 2146, chance = 12060}, -- small sapphire
	{id = 2547, chance = 6080}, -- power bolt
	{id = 2168, chance = 6030}, -- life ring
	{id = 2167, chance = 5950}, -- energy ring
	{id = 2033, chance = 2940}, -- golden mug
	{id = 7386, chance = 2040}, -- mercenary sword
	{id = 7590, chance = 2030}, -- great mana potion
	{id = 2182, chance = 990}, -- snakebite rod
	{id = 11230, chance = 950}, -- winged tail
	{id = 2528, chance = 860}, -- tower shield
	{id = 7456, chance = 810}, -- noble axe
	{id = 2177, chance = 790}, -- life crystal
	{id = 2479, chance = 640}, -- strange helmet
	{id = 2475, chance = 550}, -- warrior helmet
	{id = 5956, chance = 550}, -- old parchment
	{id = 2487, chance = 520}, -- crown armor
	{id = 3971, chance = 180}, -- charmer's tiara
	{id = 2498, chance = 120}, -- royal helmet
	{id = 8902, chance = 90}, -- spellbook of mind control
	{id = 8880, chance = 80}, -- swamplair armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -286, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -50, maxDamage = -500, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 2000, chance = 25, range = 7, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENBUBBLE, target = true, speed = -750, duration = 15000},
	{name = "combat", interval = 2000, chance = 10, minDamage = -150, maxDamage = -400, effect = CONST_ME_REDNOTE, target = false, length = 8, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -300, effect = CONST_ME_POISON, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 35,
	{name = "combat", interval = 2000, chance = 15, minDamage = 300, maxDamage = 400, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 340, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)