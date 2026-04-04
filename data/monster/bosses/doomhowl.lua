local mType = Game.createMonsterType("Doomhowl")
local monster = {}

monster.description = "Doomhowl"
monster.experience = 3750
monster.outfit = {
	lookType = 308,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 8500
monster.maxHealth = 8500
monster.race = "blood"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 0

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
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 2000, chance = 50, minDamage = 0, maxDamage = -645, radius = 3, effect = CONST_ME_REDSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 4000, chance = 20, radius = 0, effect = CONST_ME_GREENNOTE, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 55,
	armor = 50,
	{name = "speed", interval = 2000, chance = 10, effect = CONST_ME_REDSHIMMER, speed = 390, duration = 6000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)