local mType = Game.createMonsterType("Haunted Treeling")
local monster = {}

monster.description = "a haunted treeling"
monster.experience = 310
monster.outfit = {
	lookType = 310,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9867
monster.health = 450
monster.maxHealth = 450
monster.race = "undead"
monster.speed = 230
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
	targetDistance = 1,
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Knarrrz", yell = false},
	{text = "Huuhuuhuuuhuuaarrr", yell = false},
	{text = "Knorrrrrr", yell = false},
}

monster.loot = {
	{id = 2148, chance = 91920, maxCount = 95}, -- gold coin
	{id = 2788, chance = 7700}, -- red mushroom
	{id = 7618, chance = 5130}, -- health potion
	{id = 2787, chance = 5030, maxCount = 2}, -- white mushroom
	{id = 10600, chance = 4950}, -- haunted piece of wood
	{id = 2790, chance = 1800}, -- orange mushroom
	{id = 7588, chance = 1040}, -- strong health potion
	{id = 2213, chance = 660}, -- dwarven ring
	{id = 2149, chance = 620}, -- small emerald
	{id = 7443, chance = 100}, -- bullseye potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 2000, chance = 5, minDamage = -30, maxDamage = -100, radius = 4, effect = CONST_ME_GREENBUBBLE, target = false, type = COMBAT_MANADRAIN},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_SMALLPLANTS, target = false, length = 5, spread = 0, speed = -700, duration = 15000},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -100, range = 7, radius = 1, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_CARNIPHILA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -55, maxDamage = -100, radius = 4, effect = CONST_ME_GREENSPARK, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, radius = 1, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = 1},
	{type = COMBAT_ICEDAMAGE, percent = 15},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)