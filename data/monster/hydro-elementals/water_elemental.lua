local mType = Game.createMonsterType("Water Elemental")
local monster = {}

monster.description = "a water elemental"
monster.experience = 650
monster.outfit = {
	lookType = 286,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 10499
monster.health = 550
monster.maxHealth = 550
monster.race = "undead"
monster.speed = 230
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Splish splash", yell = false},
}

monster.loot = {
	{id = 2145, chance = 1000}, -- small diamond
	{id = 2146, chance = 1000}, -- small sapphire
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2149, chance = 1000, maxCount = 2}, -- small emerald
	{id = 2152, chance = 10000}, -- platinum coin
	{id = 2167, chance = 950}, -- energy ring
	{id = 2168, chance = 930}, -- life ring
	{id = 2667, chance = 20000}, -- fish
	{id = 7158, chance = 940}, -- rainbow trout
	{id = 7159, chance = 1050}, -- green perch
	{id = 7588, chance = 10000}, -- strong health potion
	{id = 7589, chance = 10000}, -- strong mana potion
	{id = 7632, chance = 800},
	{id = 7633, chance = 800},
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -160, interval = 2000, target = false},
	{name = "combat", type = COMBAT_DROWNDAMAGE, minDamage = -125, maxDamage = -235, interval = 2000, chance = 10, range = 7, radius = 2, target = true, effect = CONST_ME_BLUEBUBBLE},
	{name = "combat", type = COMBAT_ICEDAMAGE, minDamage = -88, maxDamage = -150, interval = 2000, chance = 20, range = 7, target = true, shootEffect = CONST_ANI_SMALLICE},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -225, maxDamage = -260, radius = 5, effect = CONST_ME_POISON, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 37,
	{name = "combat", interval = 2000, chance = 5, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 35},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)