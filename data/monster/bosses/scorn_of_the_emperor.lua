local mType = Game.createMonsterType("Scorn Of The Emperor")
local monster = {}

monster.description = "a scorn of the emperor"
monster.experience = 450
monster.outfit = {
	lookType = 351,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 12317
monster.health = 45000
monster.maxHealth = 45000
monster.race = "undead"
monster.speed = 410
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 366,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 55, attack = 115, target = false},
	{name = "combat", interval = 3000, chance = 17, minDamage = -150, maxDamage = -250, effect = CONST_ME_BLUEBUBBLE, target = false, length = 8, spread = 3, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 3000, chance = 10, minDamage = 0, maxDamage = -500, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 1000, chance = 10, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -600, duration = 20000},
	{name = "combat", interval = 2000, chance = 21, minDamage = -200, maxDamage = -450, radius = 6, effect = CONST_ME_POFF, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 35,
	armor = 45,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}

monster.summons = {
	{name = "Draken Warmaster", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)