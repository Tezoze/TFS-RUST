local mType = Game.createMonsterType("Cublarc the Plunderer")
local monster = {}

monster.description = "Clubarc The Plunderer"
monster.experience = 400
monster.outfit = {
	lookType = 342,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11254
monster.health = 400
monster.maxHealth = 400
monster.race = "blood"
monster.speed = 210
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
	canPushCreatures = true,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Orc arga Huummmak!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 58000, maxCount = 78}, -- gold coin
	{id = 2666, chance = 24600}, -- meat
	{id = 2428, chance = 21000}, -- orcish axe
	{id = 11324, chance = 13000}, -- shaggy tail
	{id = 11338, chance = 6000}, -- disgusting trophy
	{id = 2456, chance = 4600}, -- bow
	{id = 8857, chance = 3070}, -- silkweaver bow
	{id = 11113, chance = 1500}, -- orc tooth
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -130, interval = 2000, target = false},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = 0, maxDamage = -85, interval = 2000, chance = 50, range = 7, target = true, shootEffect = CONST_ANI_ONYXARROW},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 11, tick = 4000, minDamage = -8, maxDamage = -8, effect = CONST_ME_POISON, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 350, duration = 5000},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 2},
	{type = COMBAT_EARTHDAMAGE, percent = -2},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)