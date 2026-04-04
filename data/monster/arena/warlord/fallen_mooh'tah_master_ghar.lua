local mType = Game.createMonsterType("Fallen Mooh'tah Master Ghar")
local monster = {}

monster.description = "Fallen Mooh'Tah Master Ghar"
monster.experience = 4400
monster.outfit = {
	lookType = 29,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 7990
monster.maxHealth = 7990
monster.race = "blood"
monster.speed = 190
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
	targetDistance = 1,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "I will finish you!", yell = false},
	{text = "You are no match for a master of the Mooh'Tha!", yell = false},
	{text = "I might be fallen but you will fall before me!", yell = false},
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -525, interval = 2000, target = false},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -80, maxDamage = -300, interval = 6000, chance = 30, length = 8, spread = 3, target = false, effect = CONST_ME_FIREAREA},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -80, maxDamage = -400, interval = 3000, chance = 45, radius = 5, target = true, shootEffect = CONST_ANI_FIRE},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = -80, maxDamage = -270, interval = 4000, chance = 30, range = 7, target = true, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_MORTAREA},
	{name = "condition", type = CONDITION_POISON, interval = 4500, chance = 40, tick = 4000, minDamage = -10, maxDamage = -200, range = 10, shootEffect = CONST_ANI_POISON, target = true},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -60, maxDamage = -300, interval = 5000, chance = 30, length = 8, spread = 3, target = false, effect = CONST_ME_POISON},
}

monster.defenses = {
	defense = 33,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 60},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)