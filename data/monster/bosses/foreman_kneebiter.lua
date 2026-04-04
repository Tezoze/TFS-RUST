local mType = Game.createMonsterType("Foreman Kneebiter")
local monster = {}

monster.description = "Foreman Kneebiter"
monster.experience = 445
monster.outfit = {
	lookType = 70,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6013
monster.health = 570
monster.maxHealth = 570
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	attackable = true,
	hostile = true,
	canPushItems = true,
	targetDistance = 1,
	staticAttackChance = 90,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 30,
	{text = "By Durin's beard!", yell = true},
}

monster.loot = {
	{id = 5880, chance = 2500, maxCount = 2}, -- iron ore
	{id = 2148, chance = 90000, maxCount = 100}, -- gold coin
	{id = 2513, chance = 6666}, -- battle shield
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = -60, maxDamage = -200, target = false},
}

monster.defenses = {
	defense = 22,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 90},
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)