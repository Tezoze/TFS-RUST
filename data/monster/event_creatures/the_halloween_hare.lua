local mType = Game.createMonsterType("The Halloween Hare")
local monster = {}

monster.description = "The Halloween Hare"
monster.experience = 0
monster.outfit = {
	lookType = 74,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.health = 1
monster.maxHealth = 1
monster.race = "blood"
monster.speed = 150
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 95
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 2,
	staticAttackChance = 10,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = 0, target = false},
	{name = "outfit", interval = 2000, chance = 6, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 6, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 6, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
	{name = "outfit", interval = 2000, chance = 15, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false},
}

monster.defenses = {
	defense = 999,
	armor = 999,
	{name = "combat", interval = 1000, chance = 50, minDamage = 1500, maxDamage = 2000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "physical", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)