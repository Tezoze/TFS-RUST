local mType = Game.createMonsterType("Toad")
local monster = {}

monster.description = "a toad"
monster.experience = 60
monster.outfit = {
	lookType = 222,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6077
monster.health = 135
monster.maxHealth = 135
monster.race = "blood"
monster.speed = 210
monster.manaCost = 400
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
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ribbit! Ribbit!", yell = false},
	{text = "Ribbit!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 80000, maxCount = 20}, -- gold coin
	{id = 2391, chance = 148}, -- war hammer
	{id = 2398, chance = 2854}, -- mace
	{id = 2667, chance = 20000}, -- fish
	{id = 10557, chance = 4761}, -- poisonous slime
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false, condition = {type = CONDITION_POISON, startDamage = 20, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -8, maxDamage = -17, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENBUBBLE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 6,
	armor = 6,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 200, duration = 5000},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}


mType:register(monster)