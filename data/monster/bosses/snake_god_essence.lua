local mType = Game.createMonsterType("Snake God Essence")
local monster = {}

monster.description = "Snake God Essence"
monster.experience = 7410
monster.outfit = {
	lookType = 356,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 0
monster.health = 65000
monster.maxHealth = 65000
monster.race = "blood"
monster.speed = 300
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
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 10,
	{text = "AHHH ZHE POWER...", yell = true},
	{text = "ZHE TIME OF ZHE SNAKE HAZ COME!", yell = true},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -600, target = false},
	{name = "combat", interval = 2000, chance = 40, maxDamage = -300, effect = CONST_ME_REDSHIMMER, target = false, length = 8, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 50, minDamage = -150, maxDamage = -270, radius = 6, effect = CONST_ME_GREENSHIMMER, target = false, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 65,
	armor = 70,
	{name = "combat", interval = 2000, chance = 25, minDamage = 150, maxDamage = 450, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)