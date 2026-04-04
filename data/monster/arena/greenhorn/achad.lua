local mType = Game.createMonsterType("Achad")
local monster = {}

monster.description = "Achad"
monster.experience = 70
monster.outfit = {
	lookType = 146,
	lookHead = 93,
	lookBody = 93,
	lookLegs = 57,
	lookFeet = 97,
	lookAddons = 3,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 185
monster.maxHealth = 185
monster.race = "blood"
monster.speed = 185
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
	runHealth = 55,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You won't pass me.", yell = false},
	{text = "I have travelled far to fight here.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false},
}

monster.defenses = {
	defense = 19,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)