local mType = Game.createMonsterType("Zulazza The Corruptor")
local monster = {}

monster.description = "Zulazza The Corruptor"
monster.experience = 10000
monster.outfit = {
	lookType = 334,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11107
monster.health = 46500
monster.maxHealth = 46500
monster.race = "blood"
monster.speed = 290
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
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
	runHealth = 1500,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 10,
	{text = "I'm Zulazza, and you won't forget me that fazzt.", yell = false},
	{text = "Zzaion is our last zztand! I will not leave wizzout a fight!", yell = false},
	{text = "Behind zze Great Gate liezz your doom!", yell = false},
	{text = "Oh, HE will take revenge on zzizz azzault when you zztep in front of HIZZ fazze!", yell = false},
}

monster.loot = {
	{id = 7591, chance = 30500}, -- great health potion
	{id = 2152, chance = 41325, maxCount = 30}, -- platinum coin
	{id = 2148, chance = 49650, maxCount = 100}, -- gold coin
	{id = 9808, chance = 50500},
	{id = 9971, chance = 33000, maxCount = 4}, -- gold ingot
	{id = 2158, chance = 30500}, -- blue gem
	{id = 2155, chance = 20500}, -- green gem
	{id = 8473, chance = 10500}, -- ultimate health potion
	{id = 11118, chance = 5500}, -- dragon scale boots
	{id = 5944, chance = 19250, maxCount = 4}, -- soul orb
	{id = 2528, chance = 15500}, -- tower shield
	{id = 7366, chance = 8100, maxCount = 67}, -- viper star
	{id = 7632, chance = 28000, maxCount = 2},
	{id = 2154, chance = 15500}, -- yellow gem
	{id = 2156, chance = 10500}, -- red gem
	{id = 7440, chance = 10500}, -- mastermind potion
	{id = 2153, chance = 25500}, -- violet gem
	{id = 7590, chance = 20500}, -- great mana potion
	{id = 8882, chance = 5500}, -- earthborn titan armor
	{id = 2514, chance = 5500}, -- mastermind shield
	{id = 2127, chance = 10500}, -- emerald bangle
	{id = 8891, chance = 5500}, -- paladin armor
	{id = 2515, chance = 5500}, -- guardian shield
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 200, attack = 200, target = false},
	{name = "combat", interval = 2000, chance = 40, minDamage = -500, maxDamage = -800, effect = CONST_ME_MORTAREA, target = false, length = 8, spread = 0, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 30, minDamage = -300, maxDamage = -800, radius = 3, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 25, minDamage = -50, maxDamage = -130, range = 7, effect = CONST_ME_GREENSHIMMER, target = true, type = COMBAT_MANADRAIN},
	{name = "speed", interval = 2000, chance = 20, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -500, duration = 20000},
}

monster.defenses = {
	defense = 119,
	armor = 96,
	{name = "combat", interval = 2000, chance = 20, minDamage = 2000, maxDamage = 3000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 70},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "energy", combat = true, condition = true},
}


mType:register(monster)