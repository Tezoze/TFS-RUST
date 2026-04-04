local mType = Game.createMonsterType("Avalanche")
local monster = {}

monster.description = "Avalanche"
monster.experience = 305
monster.outfit = {
	lookType = 261,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 550
monster.maxHealth = 550
monster.race = "undead"
monster.speed = 195
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
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 5,
	{text = "You will pay for imprisoning me here.", yell = false},
	{text = "Puny warmblood.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -180, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -10, maxDamage = -50, effect = CONST_ME_ENERGYAREA, target = false, length = 5, spread = 6, type = COMBAT_DROWNDAMAGE},
	{name = "speed", interval = 4000, chance = 20, radius = 3, effect = CONST_ME_POFF, target = false, speed = -400, duration = 10000},
	{name = "combat", interval = 6000, chance = 20, minDamage = 0, maxDamage = -40, range = 5, radius = 1, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 27,
	armor = 26,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)