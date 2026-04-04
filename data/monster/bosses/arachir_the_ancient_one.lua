local mType = Game.createMonsterType("Arachir The Ancient One")
local monster = {}

monster.description = "Arachir The Ancient One"
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
monster.health = 1600
monster.maxHealth = 1600
monster.race = "undead"
monster.speed = 286
monster.manaCost = 0
monster.maxSummons = 2

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
	{text = "I was the shadow that haunted the cradle of humanity!", yell = false},
	{text = "I exist since eons and you want to defy me?", yell = false},
	{text = "Can you feel the passage of time, mortal?", yell = false},
	{text = "Your worthles existence will nourish something greater!", yell = false},
}

monster.loot = {
	{id = 7416, chance = 1200}, -- bloody edge
	{id = 7588, chance = 10000}, -- strong health potion
	{id = 2229, chance = 10000}, -- skull
	{id = 2148, chance = 100000, maxCount = 98}, -- gold coin
	{id = 9020, chance = 100000}, -- vampire lord token
	{id = 2152, chance = 50000, maxCount = 5}, -- platinum coin
	{id = 2534, chance = 6300}, -- vampire shield
	{id = 2144, chance = 8980}, -- black pearl
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 70, attack = 95, target = false},
	{name = "combat", interval = 9000, chance = 100, minDamage = -120, maxDamage = -300, radius = 3, effect = CONST_ME_MORTAREA, target = false, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 1000, chance = 12, minDamage = 0, maxDamage = -120, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 1000, chance = 12, minDamage = 100, maxDamage = 235, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 3000, chance = 25, effect = CONST_ME_BLUESHIMMER},
	{name = "outfit", interval = 4500, chance = 30, monster = "bat", duration = 4000},
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
	{name = "Lich", chance = 100, interval = 9000, max = 2},
	{name = "Lich", chance = 100, interval = 9000, max = 2},
}

mType:register(monster)