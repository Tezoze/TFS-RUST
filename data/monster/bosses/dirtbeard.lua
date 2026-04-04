local mType = Game.createMonsterType("Dirtbeard")
local monster = {}

monster.description = "Dirtbeard"
monster.experience = 375
monster.outfit = {
	lookType = 98,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 630
monster.maxHealth = 630
monster.race = "blood"
monster.speed = 300
monster.manaCost = 0
monster.maxSummons = 2

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
	runHealth = 50,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "You are no match for the scourge of the seas!", yell = false},
	{text = "You move like a seasick whale!", yell = false},
	{text = "Yarr, death to all landlubbers!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 2000, maxCount = 95}, -- gold coin
	{id = 2152, chance = 30000, maxCount = 9}, -- platinum coin
	{id = 10292, chance = 1000}, -- pointed rabbitslayer
	{id = 10299, chance = 1000}, -- helmet of nature
	{id = 10291, chance = 1000}, -- odd hat
	{id = 10318, chance = 2000}, -- shield nevermourn
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -125, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = 0, maxDamage = -100, range = 7, shootEffect = CONST_ANI_THROWINGSTAR, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "melee", interval = 2000, chance = 30, range = 7, radius = 3, effect = CONST_ME_REDNOTE, target = false},
	{name = "combat", interval = 2000, chance = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Pirate Marauder", chance = 30, interval = 4000, max = 2},
}

mType:register(monster)