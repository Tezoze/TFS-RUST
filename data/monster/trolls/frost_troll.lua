local mType = Game.createMonsterType("Frost Troll")
local monster = {}

monster.description = "a frost troll"
monster.experience = 23
monster.outfit = {
	lookType = 53,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5998
monster.health = 55
monster.maxHealth = 55
monster.race = "blood"
monster.speed = 140
monster.manaCost = 300
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Brrr", yell = false},
	{text = "Broar!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50840, maxCount = 12}, -- gold coin
	{id = 2245, chance = 8300}, -- twigs
	{id = 2384, chance = 15500}, -- rapier
	{id = 2389, chance = 21500}, -- spear
	{id = 2512, chance = 15850}, -- wooden shield
	{id = 2651, chance = 1200}, -- coat
	{id = 2667, chance = 18000}, -- fish
	{id = 10565, chance = 2000}, -- frosty ear of a troll
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 6,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -15},
}


mType:register(monster)