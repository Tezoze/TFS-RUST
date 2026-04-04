local mType = Game.createMonsterType("Renegade Orc")
local monster = {}

monster.description = "a renegade orc"
monster.experience = 60
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
monster.health = 120
monster.maxHealth = 120
monster.race = "blood"
monster.speed = 180
monster.manaCost = 420
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 5
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrr!", yell = false},
	{text = "Death to all!", yell = false},
	{text = "No mercy!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 33330, maxCount = 35}, -- gold coin
	{id = 2667, chance = 27780}, -- fish
	{id = 11113, chance = 8330}, -- orc tooth
	{id = 2510, chance = 8330}, -- plate shield
	{id = 2419, chance = 8330}, -- scimitar
	{id = 2789, chance = 5560}, -- brown mushroom
	{id = 2397, chance = 5560}, -- longsword
	{id = 2647, chance = 5560}, -- plate legs
	{id = 2410, chance = 5560, maxCount = 2}, -- throwing knife
	{id = 7378, chance = 2780}, -- royal spear
	{id = 2207, chance = 2780}, -- sword ring
	{id = 2475, chance = 2780}, -- warrior helmet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -213, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -81, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
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