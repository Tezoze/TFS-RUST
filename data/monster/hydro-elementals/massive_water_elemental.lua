local mType = Game.createMonsterType("Massive Water Elemental")
local monster = {}

monster.description = "a massive water elemental"
monster.experience = 1100
monster.outfit = {
	lookType = 11,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 10499
monster.health = 1250
monster.maxHealth = 1250
monster.race = "undead"
monster.speed = 430
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

monster.loot = {
	{id = 2145, chance = 1900, maxCount = 2}, -- small diamond
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2149, chance = 1000, maxCount = 2}, -- small emerald
	{id = 2152, chance = 20000, maxCount = 2}, -- platinum coin
	{id = 2167, chance = 910}, -- energy ring
	{id = 2168, chance = 1000}, -- life ring
	{id = 2667, chance = 40000, maxCount = 2}, -- fish
	{id = 7158, chance = 1340}, -- rainbow trout
	{id = 7159, chance = 1590}, -- green perch
	{id = 7590, chance = 10400}, -- great mana potion
	{id = 7591, chance = 10000}, -- great health potion
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -220, interval = 2000, target = false},
	{name = "combat", type = COMBAT_DROWNDAMAGE, minDamage = -330, maxDamage = -450, interval = 2000, chance = 15, range = 7, radius = 2, target = true, effect = CONST_ME_BLUEBUBBLE},
	{name = "combat", type = COMBAT_ICEDAMAGE, minDamage = -170, maxDamage = -210, interval = 2000, chance = 20, range = 7, target = true, shootEffect = CONST_ANI_SMALLICE},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 20, tick = 4000, minDamage = -355, maxDamage = -420, radius = 5, effect = CONST_ME_POISON, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 45,
	{name = "combat", interval = 2000, chance = 5, minDamage = 120, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
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