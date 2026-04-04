local mType = Game.createMonsterType("Braindeath")
local monster = {}

monster.description = "a braindeath"
monster.experience = 985
monster.outfit = {
	lookType = 256,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7256
monster.health = 1225
monster.maxHealth = 1225
monster.race = "undead"
monster.speed = 218
monster.manaCost = 0
monster.maxSummons = 2

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
	targetDistance = 4,
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You have disturbed my thoughts!", yell = false},
	{text = "Let me turn you into something more useful!", yell = false},
	{text = "Let me taste your brain!", yell = false},
	{text = "You will be punished!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 99470, maxCount = 89}, -- gold coin
	{id = 2450, chance = 15130}, -- bone sword
	{id = 7364, chance = 9560, maxCount = 4}, -- sniper arrow
	{id = 2509, chance = 5940}, -- steel shield
	{id = 10580, chance = 5030}, -- piece of dead brain
	{id = 5898, chance = 2990}, -- bonelord eye
	{id = 2423, chance = 1970}, -- clerical mace
	{id = 7407, chance = 1440}, -- haunted blade
	{id = 2175, chance = 930}, -- spellbook
	{id = 7452, chance = 180}, -- spiked squelcher
	{id = 2518, chance = 100}, -- bonelord shield
	{id = 3972, chance = 100}, -- bonelord helmet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -93, maxDamage = -170, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -75, maxDamage = -125, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -85, maxDamage = -170, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -65, maxDamage = -125, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -75, maxDamage = -85, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -155, range = 7, target = false, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 12,
	armor = 12,
	{name = "combat", interval = 2000, chance = 15, minDamage = 150, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 260, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = -20},
	{type = COMBAT_FIREDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Vampire", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)