local mType = Game.createMonsterType("Inky")
local monster = {}

monster.description = "Inky"
monster.experience = 250
monster.outfit = {
	lookType = 46,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6065
monster.health = 750
monster.maxHealth = 750
monster.race = "blood"
monster.speed = 340
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Tssss!", yell = false},
	{text = "Gaaahhh!", yell = false},
	{text = "Gluh! Gluh!", yell = false},
	{text = "Boohaa!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 12000, maxCount = 13}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -156, target = false, condition = {type = CONDITION_POISON, startDamage = 2, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -87, radius = 3, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 19, minDamage = 0, maxDamage = -80, radius = 3, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 7, minDamage = -56, maxDamage = -87, radius = 4, effect = CONST_ME_ICEAREA, target = false, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, range = 1, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 90},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "drown", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)