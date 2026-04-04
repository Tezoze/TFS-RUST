local mType = Game.createMonsterType("Wailing Widow")
local monster = {}

monster.description = "a wailing widow"
monster.experience = 450
monster.outfit = {
	lookType = 347,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11310
monster.health = 850
monster.maxHealth = 850
monster.race = "venom"
monster.speed = 254
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true,
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 37}, -- gold coin
	{id = 2381, chance = 4460}, -- halberd
	{id = 2510, chance = 2854}, -- plate shield
	{id = 2796, chance = 3208}, -- green mushroom
	{id = 7618, chance = 4761}, -- health potion
	{id = 7620, chance = 4785}, -- mana potion
	{id = 11323, chance = 2210}, -- Zaoan halberd
	{id = 11328, chance = 20950}, -- widow's mandibles
	{id = 11329, chance = 900}, -- wailing widow's necklace
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -120, target = false, condition = {type = CONDITION_POISON, startDamage = 160, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, range = 7, radius = 4, effect = CONST_ME_REDNOTE, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -40, maxDamage = -70, radius = 3, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -110, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 0,
	{name = "combat", interval = 2000, chance = 5, minDamage = 70, maxDamage = 100, effect = CONST_ME_WHITENOTE, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_YELLOWNOTE, speed = 820, duration = 5000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)