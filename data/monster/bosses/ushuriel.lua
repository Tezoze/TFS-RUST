local mType = Game.createMonsterType("Ushuriel")
local monster = {}

monster.description = "Ushuriel"
monster.experience = 10000
monster.outfit = {
	lookType = 12,
	lookHead = 0,
	lookBody = 95,
	lookLegs = 19,
	lookFeet = 112,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6068
monster.health = 31500
monster.maxHealth = 31500
monster.race = "fire"
monster.speed = 440
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You can't run or hide forever!", yell = false},
	{text = "I'm the executioner of the Seven!", yell = false},
	{text = "The final punishment awaits you!", yell = false},
	{text = "The judgement is guilty! The sentence is death!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 190}, -- gold coin
	{id = 2152, chance = 20000, maxCount = 26}, -- platinum coin
	{id = 2176, chance = 16666}, -- orb
	{id = 2177, chance = 16666}, -- life crystal
	{id = 2178, chance = 20000}, -- mind stone
	{id = 2383, chance = 9090}, -- spike sword
	{id = 2392, chance = 14285}, -- fire sword
	{id = 2393, chance = 7692}, -- giant sword
	{id = 2419, chance = 11111}, -- scimitar
	{id = 2475, chance = 20000}, -- warrior helmet
	{id = 2479, chance = 8333}, -- strange helmet
	{id = 2491, chance = 6250}, -- crown helmet
	{id = 2498, chance = 20000}, -- royal helmet
	{id = 2789, chance = 50000, maxCount = 30}, -- brown mushroom
	{id = 5669, chance = 12500}, -- mysterious voodoo skull
	{id = 5741, chance = 20000}, -- skull helmet
	{id = 5880, chance = 33333}, -- iron ore
	{id = 5884, chance = 4761}, -- spirit container
	{id = 5885, chance = 5555}, -- flask of warrior's sweat
	{id = 5891, chance = 7692}, -- enchanted chicken wing
	{id = 5892, chance = 14285}, -- huge chunk of crude iron
	{id = 5925, chance = 25000, maxCount = 20}, -- hardened bone
	{id = 5954, chance = 8333, maxCount = 2}, -- demon horn
	{id = 6103, chance = 2063}, -- unholy book
	{id = 6500, chance = 100000}, -- demonic essence
	{id = 7385, chance = 10000}, -- crimson sword
	{id = 7391, chance = 25000}, -- thaian sword
	{id = 7402, chance = 8333}, -- dragon slayer
	{id = 7417, chance = 6666}, -- runed sword
	{id = 7590, chance = 20000}, -- great mana potion
	{id = 7591, chance = 20000}, -- great health potion
	{id = 8472, chance = 20000}, -- great spirit potion
	{id = 8473, chance = 20000}, -- ultimate health potion
	{id = 9808, chance = 20000},
	{id = 9971, chance = 16666}, -- gold ingot
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -1088, interval = 2000, target = false},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = -250, maxDamage = -500, interval = 1000, chance = 10, length = 10, spread = 0, target = false, effect = CONST_ME_MORTAREA},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = -30, maxDamage = -760, interval = 1000, chance = 8, radius = 5, target = true, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_MORTAREA},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -200, maxDamage = -585, interval = 2000, chance = 9, length = 8, spread = 0, target = false, effect = CONST_ME_SMALLPLANTS},
	{name = "combat", type = COMBAT_ICEDAMAGE, minDamage = 0, maxDamage = -430, interval = 1000, chance = 8, radius = 6, target = false, effect = CONST_ME_ICETORNADO},
	{name = "drunk", interval = 3000, chance = 11, radius = 6, target = false, effect = CONST_ME_PURPLENOTE},
	{name = "condition", type = CONDITION_ENERGY, interval = 2000, chance = 15, tick = 10000, minDamage = -250, maxDamage = -250, radius = 4, effect = CONST_ME_ENERGY, target = false},
}

monster.defenses = {
	defense = 45,
	armor = 50,
	{name = "combat", interval = 1000, chance = 12, minDamage = 400, maxDamage = 600, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 4, effect = CONST_ME_BLUESHIMMER, speed = 400, duration = 7000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 25},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)