local mType = Game.createMonsterType("Green Djinn")
local monster = {}

monster.description = "a green djinn"
monster.experience = 215
monster.outfit = {
	lookType = 51,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6016
monster.health = 330
monster.maxHealth = 330
monster.race = "blood"
monster.speed = 220
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
	canPushCreatures = false,
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
	{text = "I grant you a deathwish!", yell = false},
	{text = "Muahahahahaha", yell = false},
	{text = "I wish you a merry trip to hell!", yell = false},
	{text = "Good wishes are for fairytales", yell = false},
}

monster.loot = {
	{id = 1965, chance = 2280},
	{id = 2148, chance = 41000, maxCount = 70}, -- gold coin
	{id = 2148, chance = 51000, maxCount = 45}, -- gold coin
	{id = 2149, chance = 2960, maxCount = 4}, -- small emerald
	{id = 2663, chance = 140}, -- mystic turban
	{id = 2696, chance = 23500}, -- cheese
	{id = 2747, chance = 1000}, -- grave flower
	{id = 5910, chance = 5000}, -- green piece of cloth
	{id = 7378, chance = 4870, maxCount = 2}, -- royal spear
	{id = 7620, chance = 490}, -- mana potion
	{id = 12412, chance = 2210}, -- dirty turban
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -45, maxDamage = -80, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -50, maxDamage = -105, range = 7, radius = 1, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "drunk", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, duration = 5000},
	{name = "outfit", interval = 2000, chance = 1, range = 7, effect = CONST_ME_BLUESHIMMER, target = true, duration = 4000, monster = "rat"},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 80},
	{type = COMBAT_ENERGYDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -13},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)