local mType = Game.createMonsterType("Ancient Scarab")
local monster = {}

monster.description = "an ancient scarab"
monster.experience = 720
monster.outfit = {
	lookType = 79,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6021
monster.health = 1000
monster.maxHealth = 1000
monster.race = "venom"
monster.speed = 218
monster.manaCost = 0
monster.maxSummons = 3

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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 187}, -- gold coin
	{id = 2162, chance = 10390}, -- magic light wand
	{id = 2159, chance = 8070, maxCount = 2}, -- scarab coin
	{id = 10548, chance = 7130}, -- scarab pincers
	{id = 2149, chance = 6030, maxCount = 3}, -- small emerald
	{id = 2150, chance = 5930, maxCount = 4}, -- small amethyst
	{id = 2463, chance = 4940}, -- plate armor
	{id = 2135, chance = 3600}, -- scarab amulet
	{id = 2142, chance = 2480}, -- ancient amulet
	{id = 7588, chance = 1570}, -- strong health potion
	{id = 8084, chance = 960}, -- springsprout rod
	{id = 2540, chance = 520}, -- scarab shield
	{id = 7903, chance = 440}, -- terra hood
	{id = 2440, chance = 280}, -- daramian waraxe
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -130, interval = 2000, target = false, condition = {type = CONDITION_POISON, totalDamage = 56, interval = 4000}},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -15, maxDamage = -145, interval = 2000, chance = 20, range = 7, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON},
	{name = "speed", interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, speed = -700, duration = 25000},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 30, tick = 4000, minDamage = -440, maxDamage = -520, radius = 5, effect = CONST_ME_POISON, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 36,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 380, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -20},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Larva", chance = 10, interval = 2000, max = 3},
}

mType:register(monster)