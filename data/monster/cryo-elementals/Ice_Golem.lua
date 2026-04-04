local mType = Game.createMonsterType("Ice Golem")
local monster = {}

monster.description = "an ice golem"
monster.experience = 295
monster.outfit = {
	lookType = 261,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7282
monster.health = 385
monster.maxHealth = 385
monster.race = "undead"
monster.speed = 190
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 5
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 50,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Chrrr.", yell = false},
	{text = "Crrrrk.", yell = false},
	{text = "Gnarr.", yell = false},
}

monster.loot = {
	{id = 2144, chance = 1612}, -- black pearl
	{id = 2145, chance = 66}, -- small diamond
	{id = 2146, chance = 578}, -- small sapphire
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 3}, -- gold coin
	{id = 2396, chance = 400}, -- ice rapier
	{id = 2479, chance = 444}, -- strange helmet
	{id = 7290, chance = 2660}, -- shard
	{id = 7441, chance = 5000}, -- ice cube
	{id = 7449, chance = 177}, -- crystal sword
	{id = 7588, chance = 444}, -- strong health potion
	{id = 7902, chance = 111}, -- glacier mask
	{id = 10578, chance = 11111}, -- frosty heart
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -220, target = false},
	{name = "speed", interval = 1000, chance = 13, effect = CONST_ME_ENERGY, target = false, length = 8, spread = 0, speed = -800, duration = 20000},
	{name = "combat", interval = 1000, chance = 15, minDamage = -50, maxDamage = -85, range = 7, shootEffect = CONST_ANI_SMALLICE, effect = CONST_ME_ICEATTACK, target = true, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 26,
	armor = 47,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
	{type = COMBAT_ENERGYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "holy", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)