local mType = Game.createMonsterType("Fairy")
local monster = {}

monster.description = "Fairy"
monster.experience = 0
monster.outfit = {
	lookType = 111,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6042
monster.health = 250
monster.maxHealth = 250
monster.race = "blood"
monster.speed = 250
monster.manaCost = 1000
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 50
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 3,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 4000,
	chance = 20,
	{text = "can i heal u?", yell = false},
}

monster.loot = {
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
	{name = "combat", interval = 2000, chance = 80, minDamage = -45, maxDamage = -70, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 60, minDamage = 0, maxDamage = -50, range = 5, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 50,
	{name = "combat", interval = 2000, chance = 60, minDamage = 40, maxDamage = 75, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 0},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)