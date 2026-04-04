local mType = Game.createMonsterType("Spirit of Water")
local monster = {}

monster.description = "a spirit of water"
monster.experience = 850
monster.outfit = {
	lookType = 11,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 1400
monster.maxHealth = 1400
monster.race = "undead"
monster.speed = 200
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
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 5,
	{text = "Blubb", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -400, target = false},
	{name = "poisonfield", interval = 1000, chance = 50, shootEffect = CONST_ANI_POISON, target = true},
	{name = "combat", interval = 2000, chance = 40, minDamage = -1, maxDamage = -120, effect = CONST_ME_BLUEBUBBLE, target = false, spread = 3, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 0,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)