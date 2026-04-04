local mType = Game.createMonsterType("Wyvern")
local monster = {}

monster.description = "a wyvern"
monster.experience = 515
monster.outfit = {
	lookType = 239,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6302
monster.health = 795
monster.maxHealth = 795
monster.race = "blood"
monster.speed = 186
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 300,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Shriiiek", yell = true},
}

monster.loot = {
	{id = 2127, chance = 540}, -- emerald bangle
	{id = 2146, chance = 5000}, -- small sapphire
	{id = 2148, chance = 100000, maxCount = 90}, -- gold coin
	{id = 2187, chance = 810}, -- wand of inferno
	{id = 2547, chance = 3400, maxCount = 2}, -- power bolt
	{id = 2672, chance = 60500, maxCount = 3}, -- dragon ham
	{id = 7408, chance = 410}, -- wyvern fang
	{id = 7588, chance = 2500}, -- strong health potion
	{id = 10561, chance = 12300}, -- wyvern talisman
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -120, interval = 2000, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -240, maxDamage = -240, length = 8, spread = 3, effect = CONST_ME_POISON, target = false},
	{name = "drunk", interval = 2000, chance = 10, length = 3, spread = 2, target = false, effect = CONST_ME_REDNOTE},
}

monster.defenses = {
	defense = 25,
	armor = 19,
	{name = "combat", interval = 2000, chance = 15, minDamage = 45, maxDamage = 65, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_GREENSHIMMER, speed = 300, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)