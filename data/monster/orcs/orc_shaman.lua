local mType = Game.createMonsterType("Orc Shaman")
local monster = {}

monster.description = "an orc shaman"
monster.experience = 110
monster.outfit = {
	lookType = 6,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5978
monster.health = 115
monster.maxHealth = 115
monster.race = "blood"
monster.speed = 140
monster.manaCost = 0
monster.maxSummons = 4

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
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Huumans stinkk!", yell = false},
	{text = "Grak brrretz gulu.", yell = false},
}

monster.loot = {
	{id = 1958, chance = 520},
	{id = 2148, chance = 90000, maxCount = 5}, -- gold coin
	{id = 2188, chance = 1000}, -- wand of decay
	{id = 2389, chance = 4850}, -- spear
	{id = 2464, chance = 8750}, -- chain armor
	{id = 2686, chance = 10600, maxCount = 2}, -- corncob
	{id = 11113, chance = 2100}, -- orc tooth
	{id = 12408, chance = 10300}, -- broken shamanic staff
	{id = 12434, chance = 6860}, -- shamanic hood
	{id = 12435, chance = 4300}, -- orc leather
	{id = 1950, chance = 1000}, -- book
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -15, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -20, maxDamage = -31, range = 7, shootEffect = CONST_ANI_ENERGYBALL, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -5, maxDamage = -43, range = 7, radius = 1, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 8,
	{name = "combat", interval = 2000, chance = 60, minDamage = 27, maxDamage = 43, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 25},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Snake", chance = 20, interval = 2000, max = 4},
}

mType:register(monster)