local mType = Game.createMonsterType("Dracola")
local monster = {}

monster.description = "Dracola"
monster.experience = 11000
monster.outfit = {
	lookType = 231,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6307
monster.health = 16200
monster.maxHealth = 16200
monster.race = "undead"
monster.speed = 370
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Your new name is breakfast.", yell = true},
	{text = "I'm bad to the bone.", yell = true},
	{text = "DEATH CAN'T STOP MY HUNGER!", yell = true},
}

monster.loot = {
	{id = 5944, chance = 100000}, -- soul orb
	{id = 5741, chance = 9000}, -- skull helmet
	{id = 7420, chance = 3000}, -- reaper's axe
	{id = 2177, chance = 12000}, -- life crystal
	{id = 5925, chance = 5000, maxCount = 3}, -- hardened bone
	{id = 7590, chance = 9000, maxCount = 4}, -- great mana potion
	{id = 7591, chance = 9000, maxCount = 4}, -- great health potion
	{id = 6300, chance = 14000}, -- death ring
	{id = 2489, chance = 29000}, -- dark armor
	{id = 2148, chance = 29000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 29000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 20000, maxCount = 8}, -- platinum coin
	{id = 6500, chance = 6000, maxCount = 4}, -- demonic essence
	{id = 6546, chance = 100000}, -- dracola's eye
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -700, interval = 2000, target = false},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -800, maxDamage = -1000, interval = 3000, chance = 20, length = 8, spread = 3, target = false, effect = CONST_ME_GREENSHIMMER},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -120, maxDamage = -750, interval = 2000, chance = 20, range = 7, radius = 4, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON},
	{name = "condition", type = CONDITION_DROWN, interval = 1000, chance = 20, tick = 5000, minDamage = -80, maxDamage = -80, duration = 20000, length = 8, spread = 3, effect = CONST_ME_POFF, target = false},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = -300, maxDamage = -870, interval = 2000, chance = 20, range = 7, radius = 4, target = true, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_REDSPARK},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = 0, maxDamage = -750, interval = 3000, chance = 10, range = 7, target = true, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -50, maxDamage = -175, interval = 1000, chance = 23, range = 7, radius = 4, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON},
	{name = "combat", type = COMBAT_MANADRAIN, minDamage = -100, maxDamage = -200, interval = 2000, chance = 10, range = 7, target = true},
}

monster.defenses = {
	defense = 39,
	armor = 40,
	{name = "combat", interval = 4000, chance = 10, minDamage = 500, maxDamage = 1000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)