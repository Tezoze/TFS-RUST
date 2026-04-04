local mType = Game.createMonsterType("Bones")
local monster = {}

monster.description = "Bones"
monster.experience = 3750
monster.outfit = {
	lookType = 231,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6306
monster.health = 9500
monster.maxHealth = 9500
monster.race = "undead"
monster.speed = 300
monster.manaCost = 0
monster.maxSummons = 0

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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Your new name is breakfast.", yell = false},
	{text = "Keep that dog away!", yell = false},
	{text = "Out Fluffy! Out! Bad dog!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 90}, -- gold coin
	{id = 2207, chance = 10000}, -- sword ring
	{id = 2413, chance = 4000}, -- broadsword
	{id = 2472, chance = 2000}, -- magic plate armor
	{id = 4851, chance = 800}, -- spectral stone
	{id = 5741, chance = 50000}, -- skull helmet
	{id = 5944, chance = 10000}, -- soul orb
	{id = 6300, chance = 4000}, -- death ring
	{id = 6500, chance = 1538}, -- demonic essence
	{id = 6570, chance = 5538, maxCount = 3},
	{id = 6571, chance = 1538},
	{id = 7430, chance = 50000}, -- dragonbone staff
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -845, target = false},
	{name = "combat", interval = 1000, chance = 13, minDamage = -400, maxDamage = -600, radius = 1, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 3000, chance = 34, minDamage = -180, maxDamage = -500, range = 1, radius = 1, shootEffect = CONST_ANI_DEATH, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 55,
	armor = 50,
	{name = "combat", interval = 5000, chance = 25, minDamage = 60, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)