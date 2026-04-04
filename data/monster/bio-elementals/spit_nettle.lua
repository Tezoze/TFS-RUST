local mType = Game.createMonsterType("Spit Nettle")
local monster = {}

monster.description = "a spit nettle"
monster.experience = 20
monster.outfit = {
	lookType = 221,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6062
monster.health = 150
monster.maxHealth = 150
monster.race = "venom"
monster.speed = 78
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 20
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 10750, maxCount = 5}, -- gold coin
	{id = 2804, chance = 11080}, -- shadow herb
	{id = 12432, chance = 9620}, -- nettle spit
	{id = 2802, chance = 5680, maxCount = 2}, -- sling herb
	{id = 11231, chance = 970}, -- nettle blossom
}

monster.attacks = {
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -15, maxDamage = -40, interval = 1000, chance = 20, range = 7, target = true, shootEffect = CONST_ANI_POISON},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -40, maxDamage = -100, range = 7, shootEffect = CONST_ANI_POISON, target = true},
}

monster.defenses = {
	defense = 0,
	armor = 12,
	{name = "combat", interval = 2000, chance = 10, minDamage = 8, maxDamage = 16, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)