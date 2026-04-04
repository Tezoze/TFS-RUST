local mType = Game.createMonsterType("Wild Warrior")
local monster = {}

monster.description = "a wild warrior"
monster.experience = 60
monster.outfit = {
	lookType = 131,
	lookHead = 57,
	lookBody = 57,
	lookLegs = 57,
	lookFeet = 57,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 135
monster.maxHealth = 135
monster.race = "blood"
monster.speed = 190
monster.manaCost = 420
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
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
	{text = "An enemy!", yell = false},
	{text = "Gimme your money!", yell = false},
}

monster.loot = {
	{id = 2110, chance = 520}, -- doll
	{id = 2148, chance = 49070, maxCount = 30}, -- gold coin
	{id = 2386, chance = 30710}, -- axe
	{id = 2398, chance = 9800}, -- mace
	{id = 2458, chance = 5250}, -- chain helmet
	{id = 2459, chance = 580}, -- iron helmet
	{id = 2465, chance = 2540}, -- brass armor
	{id = 2509, chance = 910}, -- steel shield
	{id = 2511, chance = 17000}, -- brass shield
	{id = 2695, chance = 9730, maxCount = 2}, -- egg
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 8,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 200, duration = 5000},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)