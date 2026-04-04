local mType = Game.createMonsterType("Webster")
local monster = {}

monster.description = "Webster"
monster.experience = 1200
monster.outfit = {
	lookType = 263,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 2950
monster.maxHealth = 2950
monster.race = "undead"
monster.speed = 290
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
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "You are lost!", yell = false},
	{text = "Come my little morsel.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
	{name = "speed", interval = 3500, chance = 65, range = 1, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, speed = -500, duration = 40},
	{name = "combat", interval = 3000, chance = 75, minDamage = -13, maxDamage = -80, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 34,
	armor = 29,
	{name = "speed", interval = 5000, chance = 100, effect = CONST_ME_REDSHIMMER, speed = 500, duration = 2500},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)