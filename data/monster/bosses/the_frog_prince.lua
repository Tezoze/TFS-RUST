local mType = Game.createMonsterType("The Frog Prince")
local monster = {}

monster.description = "the Frog Prince"
monster.experience = 1
monster.outfit = {
	lookType = 224,
	lookHead = 69,
	lookBody = 66,
	lookLegs = 69,
	lookFeet = 66,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6079
monster.health = 55
monster.maxHealth = 55
monster.race = "venom"
monster.speed = 230
monster.manaCost = 250
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 20
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Don't Kill me!!", yell = false},
	{text = "Have mercy!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 10, attack = 1, target = false},
}

monster.defenses = {
	defense = 2,
	armor = 3,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 85},
	{type = COMBAT_ICEDAMAGE, percent = 90},
	{type = COMBAT_FIREDAMAGE, percent = 10},
}


mType:register(monster)