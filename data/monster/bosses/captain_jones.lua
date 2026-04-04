local mType = Game.createMonsterType("Captain Jones")
local monster = {}

monster.description = "Captain Jones"
monster.experience = 620
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
monster.health = 555
monster.maxHealth = 555
monster.race = "undead"
monster.speed = 170
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
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 150}, -- gold coin
	{id = 2165, chance = 33000}, -- stealth ring
	{id = 2488, chance = 5070}, -- crown legs
	{id = 8871, chance = 3070}, -- focus cape
	{id = 2655, chance = 1110}, -- red robe
	{id = 2383, chance = 1110}, -- spike sword
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -95, target = false, condition = {type = CONDITION_POISON, startDamage = 2, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -80, radius = 1, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -130, maxDamage = -150, range = 1, radius = 1, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "outfit", interval = 2000, chance = 5, range = 3, shootEffect = CONST_ANI_EXPLOSION, target = true},
}

monster.defenses = {
	defense = 0,
	armor = 0,
	{name = "combat", interval = 2000, chance = 5, minDamage = 40, maxDamage = 70, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "physical", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)