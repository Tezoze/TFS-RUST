local mType = Game.createMonsterType("Kreebosh the Exile")
local monster = {}

monster.description = "Kreebosh the Exile"
monster.experience = 350
monster.outfit = {
	lookType = 103,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 705
monster.maxHealth = 705
monster.race = "blood"
monster.speed = 270
monster.manaCost = 0
monster.maxSummons = 2

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
	interval = 5000,
	chance = 5,
	{text = "I bet you wish you weren't here.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 6000, chance = 80, minDamage = 0, maxDamage = -120, radius = 3, effect = CONST_ME_ENERGY, target = false, type = COMBAT_FIREDAMAGE},
	{name = "speed", interval = 3500, chance = 35, range = 5, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, speed = -450, duration = 20000},
	{name = "combat", interval = 6000, chance = 40, minDamage = -20, maxDamage = -100, range = 5, radius = 1, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 5000, chance = 20, minDamage = -40, maxDamage = -200, range = 5, radius = 1, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 20, range = 5, radius = 1, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "outfit", interval = 2000, chance = 50, range = 5, radius = 1, effect = CONST_ME_GREENSHIMMER, target = true},
}

monster.defenses = {
	defense = 40,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 55},
	{type = COMBAT_DEATHDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Green Djinn", chance = 20, interval = 5000, max = 2},
}

mType:register(monster)