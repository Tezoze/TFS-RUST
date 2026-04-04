local mType = Game.createMonsterType("Bog Raider")
local monster = {}

monster.description = "a bog raider"
monster.experience = 800
monster.outfit = {
	lookType = 299,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8951
monster.health = 1300
monster.maxHealth = 1300
monster.race = "venom"
monster.speed = 250
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
	staticAttackChance = 60,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Tchhh!", yell = false},
	{text = "Slurp!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 92090, maxCount = 105}, -- gold coin
	{id = 2647, chance = 200}, -- plate legs
	{id = 7591, chance = 2030}, -- great health potion
	{id = 8472, chance = 2010}, -- great spirit potion
	{id = 8473, chance = 750}, -- ultimate health potion
	{id = 8872, chance = 580}, -- belted cape
	{id = 8891, chance = 140}, -- paladin armor
	{id = 8912, chance = 1020}, -- springsprout rod
	{id = 10584, chance = 9870}, -- boggy dreads
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -183, target = false, condition = {type = CONDITION_POISON, startDamage = 80, interval = 2000}},
	{name = "combat", interval = 3000, chance = 15, minDamage = -90, maxDamage = -140, range = 7, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 4000, chance = 12, minDamage = -100, maxDamage = -175, radius = 3, effect = CONST_ME_BUBBLES, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2500, chance = 18, minDamage = -96, maxDamage = -110, range = 7, shootEffect = CONST_ANI_SMALLEARTH, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 5000, chance = 20, range = 7, effect = CONST_ME_SMALLPLANTS, target = true, speed = -600, duration = 15000},
}

monster.defenses = {
	defense = 0,
	armor = 20,
	{name = "combat", interval = 2000, chance = 10, minDamage = 65, maxDamage = 95, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_EARTHDAMAGE, percent = 30},
	{type = COMBAT_FIREDAMAGE, percent = 85},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)