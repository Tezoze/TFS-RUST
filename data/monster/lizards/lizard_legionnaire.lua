local mType = Game.createMonsterType("Lizard Legionnaire")
local monster = {}

monster.description = "a lizard legionnaire"
monster.experience = 990
monster.outfit = {
	lookType = 338,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11276
monster.health = 1400
monster.maxHealth = 1400
monster.race = "blood"
monster.speed = 266
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Tssss!", yell = false},
}

monster.loot = {
	{id = 2145, chance = 1001, maxCount = 2}, -- small diamond
	{id = 2148, chance = 44000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 54000, maxCount = 65}, -- gold coin
	{id = 5876, chance = 970}, -- lizard leather
	{id = 5881, chance = 3980, maxCount = 3}, -- lizard scale
	{id = 7588, chance = 3880}, -- strong health potion
	{id = 11206, chance = 530}, -- red lantern
	{id = 11245, chance = 1950}, -- bunch of ripe rice
	{id = 11301, chance = 70}, -- Zaoan armor
	{id = 11303, chance = 460}, -- Zaoan shoes
	{id = 11305, chance = 710}, -- drakinata
	{id = 11323, chance = 960}, -- Zaoan halberd
	{id = 11334, chance = 1940}, -- legionnaire flags
	{id = 11335, chance = 14940}, -- broken halberd
	{id = 11336, chance = 20}, -- lizard trophy
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -180, target = false},
	{name = "combat", interval = 2000, chance = 40, minDamage = 0, maxDamage = -200, range = 7, shootEffect = CONST_ANI_SPEAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 45},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)