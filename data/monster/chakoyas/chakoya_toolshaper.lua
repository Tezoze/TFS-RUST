local mType = Game.createMonsterType("Chakoya Toolshaper")
local monster = {}

monster.description = "a chakoya toolshaper"
monster.experience = 40
monster.outfit = {
	lookType = 259,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7320
monster.health = 80
monster.maxHealth = 80
monster.race = "blood"
monster.speed = 136
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Chikuva!", yell = false},
	{text = "Jinuma jamjam!", yell = false},
	{text = "Suvituka siq chuqua!", yell = false},
	{text = "Kiyosa sipaju!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 77810, maxCount = 20}, -- gold coin
	{id = 2398, chance = 5300}, -- mace
	{id = 2541, chance = 720}, -- bone shield
	{id = 2553, chance = 1100}, -- pick
	{id = 2667, chance = 25060, maxCount = 2}, -- fish
	{id = 2669, chance = 2040}, -- northern pike
	{id = 7158, chance = 2040}, -- rainbow trout
	{id = 7159, chance = 2110}, -- green perch
	{id = 7381, chance = 160}, -- mammoth whopper
	{id = 7441, chance = 450}, -- ice cube
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -45, range = 7, radius = 3, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 7,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 40},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)