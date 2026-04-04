local mType = Game.createMonsterType("Servant Golem")
local monster = {}

monster.description = "a servant golem"
monster.experience = 5
monster.outfit = {
	lookType = 304,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9801
monster.health = 100
monster.maxHealth = 100
monster.race = "energy"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
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
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Do you think I might become a real boy one day?", yell = false},
	{text = "How may I serve you, Sir or Ma'am?", yell = false},
	{text = "Washing procedure initiated!", yell = false},
	{text = "Can I help you?", yell = false},
	{text = "Scan result: dusty human! Cleansing initiated!", yell = false},
	{text = "I am listening!", yell = false},
	{text = "Awaiting orders!", yell = false},
	{text = "Where are we going, Sir or Ma'am?", yell = false},
	{text = "How are you, Sir or Ma'am?", yell = false},
	{text = "Praise the Yalahari!", yell = false},
	{text = "Is that love or do you have a magnet in your pocket?", yell = false},
	{text = "Move on! There's nothing to see!", yell = false},
	{text = "Do you want to taste a sample of the newest dish?", yell = false},
	{text = "I hope I am not annoying, Sir or Ma'am?", yell = false},
	{text = "WARNING: BAD HAIRCUT DETECTED! Initializing haircut procedure!", yell = false},
	{text = "Warning: This human is extremely handsome!", yell = false},
	{text = "Mommy?", yell = false},
	{text = "Everything is working as intended!", yell = false},
	{text = "Rrrtttarrrttarrrtta", yell = false},
}

monster.attacks = {
	{name = "speed", interval = 2000, chance = 10, effect = CONST_ME_POFF, target = false, length = 8, spread = 0, speed = 300, duration = 1000},
}

monster.defenses = {
	defense = 999,
	armor = 999,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_PURPLEENERGY, speed = 240, duration = 5000},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "holy", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)