local mType = Game.createMonsterType("Acolyte of the Cult")
local monster = {}

monster.description = "an acolyte of the cult"
monster.experience = 300
monster.outfit = {
	lookType = 194,
	lookHead = 114,
	lookBody = 121,
	lookLegs = 121,
	lookFeet = 57,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 390
monster.maxHealth = 390
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 1

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
	targetDistance = 4,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Praise the voodoo!", yell = false},
	{text = "Power to the cult!", yell = false},
	{text = "Feel the power of the cult!", yell = false},
}

monster.loot = {
	{id = 1962, chance = 730},
	{id = 2148, chance = 66940, maxCount = 40}, -- gold coin
	{id = 2149, chance = 550}, -- small emerald
	{id = 2168, chance = 560}, -- life ring
	{id = 2181, chance = 250}, -- terra rod
	{id = 2201, chance = 1050}, -- dragon necklace
	{id = 2394, chance = 4990}, -- morning star
	{id = 5810, chance = 1060}, -- pirate voodoo doll
	{id = 6088, chance = 2500}, -- music sheet
	{id = 10556, chance = 8070}, -- cultish robe
	{id = 12411, chance = 40}, -- cultish symbol
	{id = 12448, chance = 10420}, -- rope belt
	{id = 12608, chance = 60}, -- broken key ring
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false, condition = {type = CONDITION_POISON, startDamage = 2, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -60, maxDamage = -120, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 5, range = 7, radius = 1, shootEffect = CONST_ANI_HOLY, effect = CONST_ME_HOLYDAMAGE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 30,
	{name = "combat", interval = 2000, chance = 15, minDamage = 40, maxDamage = 60, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Skeleton", chance = 10, interval = 2000, max = 1},
}

mType:register(monster)