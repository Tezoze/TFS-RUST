local mType = Game.createMonsterType("Stalker")
local monster = {}

monster.description = "a stalker"
monster.experience = 90
monster.outfit = {
	lookType = 128,
	lookHead = 78,
	lookBody = 116,
	lookLegs = 95,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 120
monster.maxHealth = 120
monster.race = "blood"
monster.speed = 260
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 13000, maxCount = 8}, -- gold coin
	{id = 2260, chance = 8670}, -- blank rune
	{id = 2410, chance = 11170, maxCount = 2}, -- throwing knife
	{id = 2412, chance = 530}, -- katana
	{id = 2425, chance = 1210}, -- obsidian lance
	{id = 2478, chance = 3500}, -- brass legs
	{id = 2478, chance = 5510}, -- brass legs
	{id = 12430, chance = 1550}, -- miraculum
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
	{name = "combat", interval = 1000, chance = 15, minDamage = -20, maxDamage = -30, range = 1, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 14,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)