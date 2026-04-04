local mType = Game.createMonsterType("Souleater")
local monster = {}

monster.description = "a souleater"
monster.experience = 1300
monster.outfit = {
	lookType = 355,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 12631
monster.health = 1100
monster.maxHealth = 1100
monster.race = "undead"
monster.speed = 210
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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Life is such a fickle thing!", yell = false},
	{text = "I will devour your soul.", yell = false},
	{text = "Souuuls!", yell = false},
	{text = "I will feed on you.", yell = false},
	{text = "Aaaahh", yell = false},
}

monster.loot = {
	{id = 2148, chance = 87260, maxCount = 197}, -- gold coin
	{id = 2152, chance = 50640, maxCount = 6}, -- platinum coin
	{id = 12636, chance = 15090}, -- lizard essence
	{id = 8473, chance = 8480}, -- ultimate health potion
	{id = 7590, chance = 7360}, -- great mana potion
	{id = 12637, chance = 1970}, -- ectoplasmic sushi
	{id = 2189, chance = 1410}, -- wand of cosmic energy
	{id = 2185, chance = 960}, -- necrotic rod
	{id = 6300, chance = 330}, -- death ring
	{id = 5884, chance = 210}, -- spirit container
	{id = 2197, chance = 140}, -- stone skin amulet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -209, target = false},
	{name = "combat", interval = 2000, chance = 100, minDamage = -50, maxDamage = -130, range = 7, shootEffect = CONST_ANI_SMALLICE, target = true, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -200, effect = CONST_ME_REDNOTE, target = false, length = 4, spread = 3, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 25, minDamage = 0, maxDamage = -60, radius = 4, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
	{name = "combat", interval = 2000, chance = 15, minDamage = 75, maxDamage = 202, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 70},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "death", combat = true, condition = true},
}


mType:register(monster)