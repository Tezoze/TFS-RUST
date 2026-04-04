local mType = Game.createMonsterType("Gnarlhound")
local monster = {}

monster.description = "a gnarlhound"
monster.experience = 60
monster.outfit = {
	lookType = 341,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11250
monster.health = 198
monster.maxHealth = 198
monster.race = "blood"
monster.speed = 410
monster.manaCost = 465
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Gnarllll!", yell = false},
	{text = "Grrrrrr!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 48000, maxCount = 30}, -- gold coin
	{id = 2666, chance = 39075, maxCount = 3}, -- meat
	{id = 3976, chance = 33300, maxCount = 3}, -- worm
	{id = 11324, chance = 25550}, -- shaggy tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}


mType:register(monster)