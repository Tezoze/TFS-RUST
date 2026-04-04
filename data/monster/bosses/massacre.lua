local mType = Game.createMonsterType("Massacre")
local monster = {}

monster.description = "Massacre"
monster.experience = 20000
monster.outfit = {
	lookType = 1278,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.health = 32000
monster.maxHealth = 32000
monster.race = "blood"
monster.speed = 430
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 5
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "HATE! HATE! KILL! KILL!", yell = true},
	{text = "GRRAAARRRHH!", yell = true},
	{text = "GRRRR!", yell = true},
}

monster.loot = {
	{id = 2231, chance = 5880}, -- big bone
	{id = 6500, chance = 100000}, -- demonic essence
	{id = 7591, chance = 5880}, -- great health potion
	{id = 7590, chance = 5880}, -- great mana potion
	{id = 2148, chance = 94120, maxCount = 157}, -- gold coin
	{id = 2522, chance = 500}, -- great shield
	{id = 2666, chance = 88240, maxCount = 9}, -- meat
	{id = 5022, chance = 82350, maxCount = 7}, -- orichalcum pearl
	{id = 2221, chance = 64710}, -- old twig
	{id = 2152, chance = 58820, maxCount = 6}, -- platinum coin
	{id = 6540, chance = 100000}, -- piece of massacre's shell
	{id = 5944, chance = 100000}, -- soul orb
	{id = 2452, chance = 1000}, -- heavy mace
	{id = 7403, chance = 900}, -- berserker
	{id = 2466, chance = 3500}, -- golden armor
	{id = 6104, chance = 1200}, -- jewel case
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 160, attack = 200, target = false},
	{name = "combat", interval = 2000, chance = 12, minDamage = 0, maxDamage = -1100, range = 7, shootEffect = CONST_ANI_LARGEROCK, effect = CONST_ME_EXPLOSIONAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 65,
	armor = 45,
	{name = "speed", interval = 2000, chance = 8, effect = CONST_ME_REDSHIMMER, speed = 790, duration = 10000},
	{name = "combat", interval = 2000, chance = 25, minDamage = 600, maxDamage = 1090, effect = CONST_ME_FIRE, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -7},
	{type = COMBAT_HOLYDAMAGE, percent = -3},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)