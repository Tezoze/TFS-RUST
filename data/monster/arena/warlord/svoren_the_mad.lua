local mType = Game.createMonsterType("Svoren the Mad")
local monster = {}

monster.description = "Svoren the Mad"
monster.experience = 3000
monster.outfit = {
	lookType = 254,
	lookHead = 80,
	lookBody = 99,
	lookLegs = 118,
	lookFeet = 38,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 6310
monster.maxHealth = 6310
monster.race = "blood"
monster.speed = 180
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "NO mommy NO. Leave me alone!", yell = false},
	{text = "Not that tower again!", yell = false},
	{text = "The cat has grown some horns!!", yell = false},
	{text = "What was I doing here again?", yell = false},
	{text = "Are we there soon mommy?", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -525, target = false},
	{name = "speed", interval = 3500, chance = 35, range = 1, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, speed = -250, duration = 40},
}

monster.defenses = {
	defense = 27,
	armor = 25,
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)