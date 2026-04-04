local mType = Game.createMonsterType("Mad Scientist")
local monster = {}

monster.description = "a mad scientist"
monster.experience = 205
monster.outfit = {
	lookType = 133,
	lookHead = 97,
	lookBody = 0,
	lookLegs = 38,
	lookFeet = 97,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 325
monster.maxHealth = 325
monster.race = "blood"
monster.speed = 180
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
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Die in the name of Science!", yell = false},
	{text = "You will regret interrupting my studies!", yell = false},
	{text = "Let me test this!", yell = false},
	{text = "I will study your corpse!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 30000, maxCount = 65}, -- gold coin
	{id = 2148, chance = 30000, maxCount = 50}, -- gold coin
	{id = 2162, chance = 2000}, -- magic light wand
	{id = 2177, chance = 2000}, -- life crystal
	{id = 2687, chance = 1200, maxCount = 5}, -- cookie
	{id = 2787, chance = 8000, maxCount = 3}, -- white mushroom
	{id = 7440, chance = 130}, -- mastermind potion
	{id = 7618, chance = 19000}, -- health potion
	{id = 7620, chance = 19000}, -- mana potion
	{id = 7762, chance = 470}, -- small enchanted amethyst
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -20, maxDamage = -56, range = 7, radius = 3, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_POFF, target = true, type = COMBAT_DROWNDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -20, maxDamage = -36, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_ENERGY, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_SMALLPLANTS, target = true, speed = -300, duration = 2000},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 15, minDamage = 10, maxDamage = 30, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)