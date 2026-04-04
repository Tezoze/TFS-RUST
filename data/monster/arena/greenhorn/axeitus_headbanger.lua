local mType = Game.createMonsterType("Axeitus Headbanger")
local monster = {}

monster.description = "Axeitus Headbanger"
monster.experience = 140
monster.outfit = {
	lookType = 71,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 365
monster.maxHealth = 365
monster.race = "blood"
monster.speed = 80
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
	{text = "Hicks!", yell = false},
	{text = "Stand still! Both of you! hicks", yell = false},
	{text = "This victory will earn me a casket of beer.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false},
	{name = "combat", interval = 1000, chance = 80, minDamage = 0, maxDamage = -50, range = 5, radius = 1, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 18,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)