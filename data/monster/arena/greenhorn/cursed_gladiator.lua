local mType = Game.createMonsterType("Cursed Gladiator")
local monster = {}

monster.description = "a cursed gladiator"
monster.experience = 215
monster.outfit = {
	lookType = 100,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 435
monster.maxHealth = 435
monster.race = "undead"
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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Death where are you?", yell = false},
	{text = "Slay me, end my curse.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 4000, chance = 50, minDamage = 0, maxDamage = 50, range = 5, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)