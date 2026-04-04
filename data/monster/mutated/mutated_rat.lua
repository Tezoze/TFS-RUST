local mType = Game.createMonsterType("Mutated Rat")
local monster = {}

monster.description = "a mutated rat"
monster.experience = 450
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
monster.health = 550
monster.maxHealth = 550
monster.race = "blood"
monster.speed = 230
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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrrrrrrrrrrrr!", yell = false},
	{text = "Fcccccchhhhhh", yell = false},
}

monster.loot = {
	{id = 2148, chance = 38000, maxCount = 65}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 65}, -- gold coin
	{id = 2165, chance = 540}, -- stealth ring
	{id = 2229, chance = 20240}, -- skull
	{id = 2235, chance = 950}, -- mouldy cheese
	{id = 2381, chance = 2990}, -- halberd
	{id = 2510, chance = 3750}, -- plate shield
	{id = 2528, chance = 50}, -- tower shield
	{id = 2796, chance = 1390}, -- green mushroom
	{id = 2799, chance = 4920}, -- stone herb
	{id = 7618, chance = 560}, -- health potion
	{id = 8900, chance = 300}, -- spellbook of enlightenment
	{id = 10585, chance = 3800}, -- mutated rat tail
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -158, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -45, maxDamage = -85, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -80, maxDamage = -100, length = 5, spread = 0, effect = CONST_ME_POISON, target = false},
	{name = "speed", interval = 2000, chance = 10, range = 7, target = true, effect = CONST_ME_REDSHIMMER, speed = -600, duration = 30000},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -30, maxDamage = -70, interval = 2000, chance = 10, range = 7, radius = 3, target = false, effect = CONST_ME_REDSHIMMER},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -80, maxDamage = -80, range = 7, radius = 3, effect = CONST_ME_POISON, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 32,
	{name = "combat", interval = 2000, chance = 5, minDamage = 80, maxDamage = 95, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)