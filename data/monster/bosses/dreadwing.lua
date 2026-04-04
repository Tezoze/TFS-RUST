local mType = Game.createMonsterType("Dreadwing")
local monster = {}

monster.description = "Dreadwing"
monster.experience = 3750
monster.outfit = {
	lookType = 307,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9829
monster.health = 8500
monster.maxHealth = 8500
monster.race = "blood"
monster.speed = 245
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 100,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 20,
	{text = "More blood! More!", yell = true},
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -240, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -70, maxDamage = -180, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_POISON},
	{name = "combat", type = COMBAT_DROWNDAMAGE, minDamage = -130, maxDamage = -237, interval = 2000, chance = 15, radius = 6, target = false, effect = CONST_ME_WHITENOTE},
	{name = "mutated bat curse", interval = 2000, chance = 10, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -12, maxDamage = -12, length = 4, spread = 3, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 10, minDamage = 80, maxDamage = 95, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)