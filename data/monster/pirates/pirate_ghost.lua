local mType = Game.createMonsterType("Pirate Ghost")
local monster = {}

monster.description = "a pirate ghost"
monster.experience = 250
monster.outfit = {
	lookType = 196,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5566
monster.health = 275
monster.maxHealth = 275
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Yooh Ho Hooh Ho!", yell = false},
	{text = "Hell is waiting for You!", yell = false},
	{text = "It's alive!", yell = false},
	{text = "The curse! Aww the curse!", yell = false},
	{text = "You will not get my treasure!", yell = false},
}

monster.loot = {
	{id = 1951, chance = 910}, -- blank parchment
	{id = 2148, chance = 48000, maxCount = 67}, -- gold coin
	{id = 2165, chance = 650}, -- stealth ring
	{id = 2383, chance = 130}, -- spike sword
	{id = 2655, chance = 130}, -- red robe
	{id = 10601, chance = 4300}, -- tattered piece of robe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false, condition = {type = CONDITION_POISON, startDamage = 40, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -40, maxDamage = -80, radius = 1, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -50, maxDamage = -65, range = 7, radius = 3, effect = CONST_ME_REDNOTE, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 30,
	{name = "combat", interval = 2000, chance = 5, minDamage = 40, maxDamage = 70, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "physical", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)