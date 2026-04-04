local mType = Game.createMonsterType("Undead Cavebear")
local monster = {}

monster.description = "an undead cavebear"
monster.experience = 600
monster.outfit = {
	lookType = 384,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13323
monster.health = 450
monster.maxHealth = 450
monster.race = "undead"
monster.speed = 109
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrrrrrrrrrr", yell = false},
	{text = "Groooowl", yell = false},
}

monster.loot = {
	{id = 2148, chance = 9750, maxCount = 80}, -- gold coin
	{id = 13291, chance = 900}, -- maxilla maximus
	{id = 13302, chance = 2350}, -- maxilla
	{id = 13303, chance = 3150}, -- cavebear skull
}

monster.attacks = {
	{name = "melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -400, target = false},
}

monster.defenses = {
	defense = 27,
	armor = 28,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 100},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
	{type = COMBAT_DEATHDAMAGE, percent = 100},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
