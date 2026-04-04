local mType = Game.createMonsterType("The Axeorcist")
local monster = {}

monster.description = "the Axeorcist"
monster.experience = 3000
monster.outfit = {
	lookType = 8,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5980
monster.health = 5100
monster.maxHealth = 5100
monster.race = "blood"
monster.speed = 250
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 9
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 9,
	{text = "DEESTRUCTIOON!", yell = true},
	{text = "Blood! Carnage! Muhahaha!", yell = true},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -276, target = false},
	{name = "combat", interval = 3000, chance = 34, minDamage = -100, maxDamage = -230, range = 7, shootEffect = CONST_ANI_WHIRLWINDAXE, effect = CONST_ME_REDSPARK, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -200, radius = 3, effect = CONST_ME_YELLOWSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 290, duration = 6000},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)