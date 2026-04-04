local mType = Game.createMonsterType("Scarab")
local monster = {}

monster.description = "a scarab"
monster.experience = 120
monster.outfit = {
	lookType = 83,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6024
monster.health = 320
monster.maxHealth = 320
monster.race = "venom"
monster.speed = 160
monster.manaCost = 395
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 80,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 86800, maxCount = 52}, -- gold coin
	{id = 2149, chance = 413}, -- small emerald
	{id = 2150, chance = 540}, -- small amethyst
	{id = 2159, chance = 3098}, -- scarab coin
	{id = 2439, chance = 245}, -- daramian mace
	{id = 2666, chance = 40000, maxCount = 2}, -- meat
	{id = 10558, chance = 4950}, -- piece of scarab shell
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -75, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -35, range = 1, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "poisonfield", interval = 2000, chance = 10, radius = 1, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 21,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 200, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_FIREDAMAGE, percent = -18},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)