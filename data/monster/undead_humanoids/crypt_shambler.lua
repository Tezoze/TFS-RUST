local mType = Game.createMonsterType("Crypt Shambler")
local monster = {}

monster.description = "a crypt shambler"
monster.experience = 195
monster.outfit = {
	lookType = 100,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6029
monster.health = 330
monster.maxHealth = 330
monster.race = "undead"
monster.speed = 140
monster.manaCost = 580
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
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
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Aaaaahhhh!", yell = false},
	{text = "Hoooohhh!", yell = false},
	{text = "Uhhhhhhh!", yell = false},
	{text = "Chhhhhhh!", yell = false},
}

monster.loot = {
	{id = 2145, chance = 510}, -- small diamond
	{id = 2148, chance = 57000, maxCount = 55}, -- gold coin
	{id = 2227, chance = 1850}, -- rotten meat
	{id = 2230, chance = 5000}, -- bone
	{id = 2399, chance = 910, maxCount = 3}, -- throwing star
	{id = 2450, chance = 1000}, -- bone sword
	{id = 2459, chance = 2130}, -- iron helmet
	{id = 2459, chance = 2000}, -- iron helmet
	{id = 2541, chance = 1000}, -- bone shield
	{id = 3976, chance = 9000, maxCount = 10}, -- worm
	{id = 11200, chance = 5000}, -- half-digested piece of meat
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -140, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -28, maxDamage = -55, range = 1, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)