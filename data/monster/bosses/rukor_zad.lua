local mType = Game.createMonsterType("Rukor Zad")
local monster = {}

monster.description = "Rukor Zad"
monster.experience = 380
monster.outfit = {
	lookType = 152,
	lookHead = 114,
	lookBody = 95,
	lookLegs = 95,
	lookFeet = 95,
	lookAddons = 3,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 380
monster.maxHealth = 380
monster.race = "blood"
monster.speed = 215
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I can kill a man in a thousand ways. And that`s only with a spoon!", yell = false},
	{text = "You shouldn`t have come here!", yell = false},
	{text = "Haiiii!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 93210, maxCount = 50}, -- gold coin
	{id = 2399, chance = 9210, maxCount = 14}, -- throwing star
	{id = 7366, chance = 6200, maxCount = 7}, -- viper star
	{id = 2457, chance = 4190}, -- steel helmet
	{id = 2509, chance = 1940}, -- steel shield
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -170, interval = 2000, target = false},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = 0, maxDamage = -100, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_THROWINGSTAR},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = 0, maxDamage = -100, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_POISONARROW},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -8, maxDamage = -8, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true},
	{name = "drunk", interval = 3000, chance = 34, range = 7, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)