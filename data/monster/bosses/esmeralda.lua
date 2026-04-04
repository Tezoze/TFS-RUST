local mType = Game.createMonsterType("Esmeralda")
local monster = {}

monster.description = "Esmeralda"
monster.experience = 600
monster.outfit = {
	lookType = 305,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9871
monster.health = 800
monster.maxHealth = 800
monster.race = "blood"
monster.speed = 245
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
	{text = "Fcccccchhhhhh", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 170}, -- gold coin
	{id = 10585, chance = 100000}, -- mutated rat tail
	{id = 2214, chance = 100000}, -- ring of healing
	{id = 2152, chance = 95000, maxCount = 4}, -- platinum coin
	{id = 2147, chance = 68000, maxCount = 3}, -- small ruby
	{id = 2476, chance = 54000}, -- knight armor
	{id = 2528, chance = 34000}, -- tower shield
	{id = 2381, chance = 31050}, -- halberd
	{id = 2438, chance = 26000}, -- epee
	{id = 7884, chance = 8200}, -- terra mantle
	{id = 2799, chance = 6500}, -- stone herb
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -170, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = 0, maxDamage = -110, interval = 2000, chance = 30, range = 7, target = true, shootEffect = CONST_ANI_POISON},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 22, tick = 4000, minDamage = -5, maxDamage = -5, length = 6, spread = 0, effect = CONST_ME_SMALLPLANTS, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -4, maxDamage = -4, radius = 3, effect = CONST_ME_POISON, target = false},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = 0, maxDamage = -110, interval = 2000, chance = 25, radius = 3, target = false, effect = CONST_ME_REDSHIMMER},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 5, minDamage = 30, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)