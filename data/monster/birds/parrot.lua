local mType = Game.createMonsterType("Parrot")
local monster = {}

monster.description = "a parrot"
monster.experience = 0
monster.outfit = {
	lookType = 217,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6056
monster.health = 25
monster.maxHealth = 25
monster.race = "blood"
monster.speed = 320
monster.manaCost = 250
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = false,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 7,
	staticAttackChance = 0,
	runHealth = 25,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "BR? PL? SWE?", yell = false},
	{text = "Screeech!", yell = false},
	{text = "Neeewbiiieee!", yell = false},
	{text = "You advanshed, you advanshed!", yell = false},
	{text = "Hope you die and loooosh it!", yell = false},
	{text = "Hunterrr ish PK!", yell = false},
	{text = "Shhtop whining! Rraaah!", yell = false},
	{text = "I'm heeerrre! Screeeech!", yell = false},
	{text = "Leeave orrr hunted!!", yell = false},
	{text = "Blesshhh my stake! Screeech!", yell = false},
	{text = "Tarrrrp?", yell = false},
	{text = "You are corrrrupt! Corrrrupt!", yell = false},
	{text = "You powerrrrrrabuserrrrr!", yell = false},
	{text = "You advanshed, you advanshed!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -5, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 2,
}


mType:register(monster)