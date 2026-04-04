local mType = Game.createMonsterType("Training Monk")
local monster = {}

monster.description = "a training monk"
monster.experience = 0
monster.outfit = {
	lookType = 57,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 200000
monster.maxHealth = 200000
monster.race = "blood"
monster.speed = 0
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
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 50,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 0, attack = -1, target = false},
}

monster.defenses = {
	defense = 0,
	armor = 0,
	{name = "combat", interval = 2000, chance = 80, minDamage = 10000, maxDamage = 20000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)