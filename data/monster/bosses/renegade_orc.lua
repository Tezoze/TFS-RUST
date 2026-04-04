local mType = Game.createMonsterType("Renegade Orc")
local monster = {}

monster.description = "a renegade orc"
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
monster.speed = 220
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Harga puchak muhmak!", yell = false},
}

monster.loot = {
	{id = 2667, chance = 30000}, -- fish
	{id = 2148, chance = 28000, maxCount = 35}, -- gold coin
	{id = 12435, chance = 19000}, -- orc leather
	{id = 2510, chance = 10000}, -- plate shield
	{id = 2410, chance = 9850, maxCount = 4}, -- throwing knife
	{id = 2789, chance = 9650}, -- brown mushroom
	{id = 2207, chance = 3920}, -- sword ring
	{id = 2397, chance = 2800}, -- longsword
	{id = 7378, chance = 2600}, -- royal spear
	{id = 2419, chance = 2100}, -- scimitar
	{id = 11113, chance = 890}, -- orc tooth
	{id = 2413, chance = 830}, -- broadsword
	{id = 7618, chance = 550}, -- health potion
	{id = 2647, chance = 420}, -- plate legs
	{id = 2475, chance = 160}, -- warrior helmet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -130, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -50, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -2},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)