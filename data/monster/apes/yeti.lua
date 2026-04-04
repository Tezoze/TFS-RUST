local mType = Game.createMonsterType("Yeti")
local monster = {}

monster.description = "a yeti"
monster.experience = 460
monster.outfit = {
	lookType = 110,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6038
monster.health = 950
monster.maxHealth = 950
monster.race = "blood"
monster.speed = 250
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Yooodelaaahooohooo", yell = false},
	{text = "Yooodelaaaheehee", yell = false},
}

monster.loot = {
	{id = 2111, chance = 10000, maxCount = 22}, -- snowball
	{id = 2148, chance = 100000, maxCount = 60}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 40}, -- gold coin
	{id = 2644, chance = 1333}, -- bunnyslippers
	{id = 2666, chance = 33333, maxCount = 4}, -- meat
	{id = 2671, chance = 10000, maxCount = 5}, -- ham
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false},
	{name = "combat", interval = 1000, chance = 15, minDamage = 0, maxDamage = -180, range = 7, shootEffect = CONST_ANI_SNOWBALL, effect = CONST_ME_POFF, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 12, minDamage = 0, maxDamage = -175, effect = CONST_ME_POFF, target = false, length = 3, spread = 3, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 33,
	armor = 28,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)