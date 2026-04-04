local mType = Game.createMonsterType("Hydra")
local monster = {}

monster.description = "a hydra"
monster.experience = 2100
monster.outfit = {
	lookType = 121,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6048
monster.health = 2350
monster.maxHealth = 2350
monster.race = "blood"
monster.speed = 250
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
	runHealth = 300,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "FCHHHHH", yell = false},
	{text = "HISSSS", yell = false},
}

monster.loot = {
	{id = 2146, chance = 5000}, -- small sapphire
	{id = 2148, chance = 34000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 34000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 20000, maxCount = 46}, -- gold coin
	{id = 2152, chance = 48000, maxCount = 3}, -- platinum coin
	{id = 2177, chance = 570}, -- life crystal
	{id = 2195, chance = 130}, -- boots of haste
	{id = 2197, chance = 900}, -- stone skin amulet
	{id = 2214, chance = 1190}, -- ring of healing
	{id = 2475, chance = 890}, -- warrior helmet
	{id = 2476, chance = 1000}, -- knight armor
	{id = 2498, chance = 210}, -- royal helmet
	{id = 2536, chance = 270}, -- medusa shield
	{id = 2671, chance = 60000, maxCount = 4}, -- ham
	{id = 4850, chance = 1930}, -- hydra egg
	{id = 7589, chance = 380}, -- strong mana potion
	{id = 8842, chance = 4780}, -- cucumber
	{id = 11199, chance = 10120}, -- hydra head
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -270, target = false},
	{name = "speed", interval = 2000, chance = 25, range = 7, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENBUBBLE, target = true, speed = -750, duration = 15000},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -250, effect = CONST_ME_BLUEBUBBLE, target = false, length = 8, spread = 3, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -80, maxDamage = -155, range = 7, shootEffect = CONST_ANI_SMALLICE, target = true, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -66, maxDamage = -320, effect = CONST_ME_CARNIPHILA, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 27,
	{name = "combat", interval = 2000, chance = 25, minDamage = 260, maxDamage = 407, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)