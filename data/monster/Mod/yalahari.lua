local mType = Game.createMonsterType("Yalahari")
local monster = {}

monster.description = "a Yalahari"
monster.experience = 5
monster.outfit = {
	lookType = 309,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 20550
monster.health = 150
monster.maxHealth = 150
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = false,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 11,
	{text = "Welcome to Yalahar, outsider.", yell = false},
	{text = "Hail Yalahar.", yell = false},
	{text = "You can learn a lot from us.", yell = false},
	{text = "Our wisdom and knowledge are unequalled in this world.", yell = false},
	{text = "That knowledge would overburden your fragile mind.", yell = false},
	{text = "I wouldn't expect you to understand.", yell = false},
	{text = "One day Yalahar will return to its former glory.", yell = false},
}

monster.defenses = {
	defense = 0,
	armor = 0,
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "physical", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
}


mType:register(monster)