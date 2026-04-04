local mType = Game.createMonsterType("Vashresamun")
local monster = {}

monster.description = "Vashresamun"
monster.experience = 2950
monster.outfit = {
	lookType = 90,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6025
monster.health = 4000
monster.maxHealth = 4000
monster.race = "undead"
monster.speed = 340
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
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Come my maidens, we have visitors!", yell = false},
	{text = "Are you enjoying my music?", yell = false},
	{text = "If music is the food of death, drop dead.", yell = false},
	{text = "Chakka Chakka!", yell = false},
	{text = "Heheheheee!", yell = false},
}

monster.loot = {
	{id = 2072, chance = 7000}, -- lute
	{id = 2074, chance = 1500}, -- panpipes
	{id = 2124, chance = 1500}, -- crystal ring
	{id = 2139, chance = 300}, -- ancient tiara
	{id = 2143, chance = 7000}, -- white pearl
	{id = 2148, chance = 50000, maxCount = 90}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 65}, -- gold coin
	{id = 2349, chance = 100000}, -- blue note
	{id = 2445, chance = 500}, -- crystal mace
	{id = 2656, chance = 2500}, -- blue robe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false, condition = {type = CONDITION_POISON, startDamage = 65, interval = 2000}},
	{name = "combat", interval = 2000, chance = 30, minDamage = -200, maxDamage = -750, radius = 5, effect = CONST_ME_PURPLENOTE, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 35,
	armor = 20,
	{name = "combat", interval = 1000, chance = 20, minDamage = 60, maxDamage = 450, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 12, effect = CONST_ME_REDSHIMMER, speed = 350, duration = 30000},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Banshee", chance = 20, interval = 2000, max = 2},
}

mType:register(monster)