local mType = Game.createMonsterType("Pirate Cutthroat")
local monster = {}

monster.description = "a pirate cutthroat"
monster.experience = 175
monster.outfit = {
	lookType = 96,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 325
monster.maxHealth = 325
monster.race = "blood"
monster.speed = 214
monster.manaCost = 495
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 15
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Give up!", yell = false},
	{text = "Hiyaa!", yell = false},
	{text = "Plundeeeeer!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 78000, maxCount = 50}, -- gold coin
	{id = 2483, chance = 3000}, -- scale armor
	{id = 2509, chance = 2800}, -- steel shield
	{id = 5091, chance = 1000}, -- treasure map
	{id = 5553, chance = 90}, -- rum flask
	{id = 5710, chance = 2000}, -- light shovel
	{id = 5792, chance = 110},
	{id = 5918, chance = 980}, -- pirate knee breeches
	{id = 5927, chance = 1000}, -- pirate bag
	{id = 6097, chance = 5500}, -- hook
	{id = 6098, chance = 4500}, -- eye patch
	{id = 6126, chance = 5000}, -- peg leg
	{id = 11219, chance = 10120}, -- compass
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -170, target = false, condition = {type = CONDITION_POISON, startDamage = 10, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -95, range = 7, radius = 1, shootEffect = CONST_ANI_EXPLOSION, effect = CONST_ME_EXPLOSIONAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)