local mType = Game.createMonsterType("Lesser Swarmer")
local monster = {}

monster.description = "a lesser swarmer"
monster.experience = 0
monster.outfit = {
	lookType = 460,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15388
monster.health = 230
monster.maxHealth = 230
monster.race = "venom"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -77, target = false, condition = {type = CONDITION_POISON, startDamage = 60, interval = 4000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -15, maxDamage = -70, effect = CONST_ME_MAGIC_RED, target = true, range = 5, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)
