local mType = Game.createMonsterType("Hot Dog")
local monster = {}

monster.description = "a hot dog"
monster.experience = 190
monster.outfit = {
	lookType = 32,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5971
monster.health = 505
monster.maxHealth = 505
monster.race = "blood"
monster.speed = 150
monster.manaCost = 220
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Wuff Wuff", yell = false},
	{text = "Grrr Wuff", yell = false},
	{text = "Show me how good you are without some rolled newspaper!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 75}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 1200, minDamage = 0, maxDamage = -55, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = -30, maxDamage = -60, effect = CONST_ME_FIRE, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 30, minDamage = -50, maxDamage = -50, range = 7, effect = CONST_ME_FIREATTACK, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 2,
	armor = 24,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)