local mType = Game.createMonsterType("Pythius The Rotten")
local monster = {}

monster.description = "Pythius The Rotten"
monster.experience = 7000
monster.outfit = {
	lookType = 231,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 9000
monster.maxHealth = 9000
monster.race = "undead"
monster.speed = 350
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	runHealth = 0,
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "YOU'LL NEVER GET MY TREASURE!", yell = true},
	{text = "MINIONS, MEET YOUR NEW BROTHER!", yell = true},
	{text = "YOU WILL REGRET THAT YOU ARE BORN!", yell = true},
	{text = "YOU MADE A HUGE WASTE!", yell = true},
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -475, interval = 2000, target = false},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = -165, maxDamage = -200, interval = 2000, chance = 16, range = 7, radius = 4, target = true, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_REDSPARK},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -55, maxDamage = -155, interval = 2000, chance = 17, range = 7, radius = 4, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -333, maxDamage = -413, interval = 2500, chance = 14, length = 8, spread = 3, target = false, effect = CONST_ME_POISON},
	{name = "combat", type = COMBAT_MANADRAIN, minDamage = -85, maxDamage = -110, interval = 2500, chance = 22, range = 7, radius = 4, target = true, shootEffect = CONST_ANI_ICE},
	{name = "speed", interval = 2000, chance = 20, range = 7, target = true, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, speed = -300, duration = 30000},
	{name = "condition", type = CONDITION_CURSED, interval = 2000, chance = 15, tick = 4000, minDamage = -60, maxDamage = -60, duration = 16000, range = 7, shootEffect = CONST_ANI_ICE, effect = CONST_ME_ICEATTACK, target = true},
}

monster.defenses = {
	defense = 45,
	armor = 40,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Undead Gladiator", chance = 10, interval = 1000, max = 2},
}

mType:register(monster)