local mType = Game.createMonsterType("Zevelon Duskbringer")
local monster = {}

monster.description = "Zevelon Duskbringer"
monster.experience = 1800
monster.outfit = {
	lookType = 287,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8937
monster.health = 1400
monster.maxHealth = 1400
monster.race = "undead"
monster.speed = 310
monster.manaCost = 0
monster.maxSummons = 3

monster.changeTarget = {
	interval = 5000,
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I want Your Blood", yell = false},
	{text = "Come Here!", yell = false},
	{text = "I will be still around when my 'noble' race is gone", yell = false},
	{text = "Human blood is not suitable for drinking!", yell = false},
	{text = "Human blood is a hardly suitable drink.", yell = false},
	{text = "Your short live is coming to an end.", yell = false},
	{text = "Ashari Mortals. Come and stay forever!", yell = false},
}

monster.loot = {
	{id = 7588, chance = 4000}, -- strong health potion
	{id = 2144, chance = 8000}, -- black pearl
	{id = 9020, chance = 100000}, -- vampire lord token
	{id = 2152, chance = 50000, maxCount = 5}, -- platinum coin
	{id = 2148, chance = 100000, maxCount = 75}, -- gold coin
	{id = 2534, chance = 4500}, -- vampire shield
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 65, attack = 75, target = false},
	{name = "combat", interval = 1000, chance = 12, minDamage = 0, maxDamage = -200, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "speed", interval = 2000, chance = 15, range = 7, target = true, speed = -700},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 1000, chance = 12, minDamage = 100, maxDamage = 235, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 3000, chance = 25, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -15},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Vampire", chance = 40, interval = 3000, max = 3},
}

mType:register(monster)