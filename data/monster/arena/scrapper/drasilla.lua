local mType = Game.createMonsterType("Drasilla")
local monster = {}

monster.description = "Drasilla"
monster.experience = 700
monster.outfit = {
	lookType = 34,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 1260
monster.maxHealth = 1260
monster.race = "blood"
monster.speed = 170
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
	runHealth = 100,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "FCHHHHHHHH!", yell = false},
	{text = "GROOOOAAAAAAAAR!", yell = false},
	{text = "DIRTY LITTLE HUMANS", yell = false},
	{text = "YOU CAN'T KEEP ME HERE FOREVER", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -120, target = false},
	{name = "combat", interval = 6000, chance = 60, minDamage = -100, maxDamage = -180, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 3000, chance = 50, minDamage = -70, maxDamage = -115, range = 10, radius = 5, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 33,
	armor = 32,
	{name = "combat", interval = 6000, chance = 65, minDamage = 20, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)