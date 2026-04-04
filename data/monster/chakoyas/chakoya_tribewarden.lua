local mType = Game.createMonsterType("Chakoya Tribewarden")
local monster = {}

monster.description = "a chakoya tribewarden"
monster.experience = 40
monster.outfit = {
	lookType = 259,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7320
monster.health = 68
monster.maxHealth = 68
monster.race = "blood"
monster.speed = 124
monster.manaCost = 305
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Quisavu tukavi!", yell = false},
	{text = "Si siyoqua jamjam!", yell = false},
	{text = "Achuq! jinuma!", yell = false},
	{text = "Si ji jusipa!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 79590, maxCount = 20}, -- gold coin
	{id = 2406, chance = 4810}, -- short sword
	{id = 2541, chance = 1030}, -- bone shield
	{id = 2667, chance = 19370}, -- fish
	{id = 2669, chance = 2040}, -- northern pike
	{id = 7158, chance = 2040}, -- rainbow trout
	{id = 7159, chance = 2110}, -- green perch
	{id = 7381, chance = 130}, -- mammoth whopper
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 9,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 25},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)