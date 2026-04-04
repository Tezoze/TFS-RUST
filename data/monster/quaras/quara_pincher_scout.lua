local mType = Game.createMonsterType("Quara Pincher Scout")
local monster = {}

monster.description = "a quara pincher scout"
monster.experience = 600
monster.outfit = {
	lookType = 77,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6063
monster.health = 775
monster.maxHealth = 775
monster.race = "blood"
monster.speed = 156
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
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Clank! Clank!", yell = false},
	{text = "Clap!", yell = false},
	{text = "Crrrk! Crrrk!", yell = false},
}

monster.loot = {
	{id = 2147, chance = 3440}, -- small ruby
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 49000, maxCount = 43}, -- gold coin
	{id = 2177, chance = 1000}, -- life crystal
	{id = 2381, chance = 1840}, -- halberd
	{id = 2463, chance = 4170}, -- plate armor
	{id = 5895, chance = 5940, maxCount = 2}, -- fish fin
	{id = 12446, chance = 9940}, -- quara pincers
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -240, target = false},
	{name = "speed", interval = 2000, chance = 20, range = 1, effect = CONST_ME_REDSHIMMER, target = false, speed = -600, duration = 3000},
}

monster.defenses = {
	defense = 45,
	armor = 70,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)