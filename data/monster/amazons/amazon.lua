local mType = Game.createMonsterType("Amazon")
local monster = {}

monster.description = "an amazon"
monster.experience = 60
monster.outfit = {
	lookType = 137,
	lookHead = 113,
	lookBody = 120,
	lookLegs = 95,
	lookFeet = 115,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 110
monster.maxHealth = 110
monster.race = "blood"
monster.speed = 172
monster.manaCost = 390
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Yeeee ha!", yell = false},
	{text = "Your head shall be mine!", yell = false},
	{text = "Your head will be mine!", yell = false},
}

monster.loot = {
	{id = 2379, chance = 80000}, -- dagger
	{id = 2229, chance = 80000, maxCount = 2}, -- skull
	{id = 2148, chance = 40000, maxCount = 20}, -- gold coin
	{id = 2691, chance = 30000}, -- brown bread
	{id = 2385, chance = 23000}, -- sabre
	{id = 12399, chance = 10000}, -- girlish hair decoration
	{id = 12400, chance = 5200}, -- protective charm
	{id = 2050, chance = 1000}, -- torch
	{id = 2125, chance = 260}, -- crystal necklace
	{id = 2147, chance = 130}, -- small ruby
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -40, range = 5, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)