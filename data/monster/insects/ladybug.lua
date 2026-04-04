local mType = Game.createMonsterType("Ladybug")
local monster = {}

monster.description = "a ladybug"
monster.experience = 70
monster.outfit = {
	lookType = 448,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15272
monster.health = 255
monster.maxHealth = 255
monster.race = "venom"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 60,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Nee pah!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 65000, maxCount = 40}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -4, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -20, shootEffect = CONST_ANI_POISON, target = true, range = 1, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -20, shootEffect = CONST_ANI_POISON, target = true, range = 7, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
