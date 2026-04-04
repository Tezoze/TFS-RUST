local mType = Game.createMonsterType("Orc Leader")
local monster = {}

monster.description = "an orc leader"
monster.experience = 270
monster.outfit = {
	lookType = 59,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6001
monster.health = 450
monster.maxHealth = 450
monster.race = "blood"
monster.speed = 230
monster.manaCost = 640
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ulderek futgyr human!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 28000, maxCount = 35}, -- gold coin
	{id = 2207, chance = 3920}, -- sword ring
	{id = 2397, chance = 2800}, -- longsword
	{id = 2410, chance = 9950, maxCount = 4}, -- throwing knife
	{id = 2413, chance = 610}, -- broadsword
	{id = 2419, chance = 1860}, -- scimitar
	{id = 2463, chance = 1650}, -- plate armor
	{id = 2475, chance = 180}, -- warrior helmet
	{id = 2478, chance = 3100}, -- brass legs
	{id = 2510, chance = 1650}, -- plate shield
	{id = 2647, chance = 440}, -- plate legs
	{id = 2667, chance = 29400}, -- fish
	{id = 2789, chance = 9650}, -- brown mushroom
	{id = 7378, chance = 2400}, -- royal spear
	{id = 7618, chance = 550}, -- health potion
	{id = 11113, chance = 1030}, -- orc tooth
	{id = 12435, chance = 19510}, -- orc leather
	{id = 12436, chance = 2008}, -- skull belt
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -185, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -70, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)