local mType = Game.createMonsterType("Monstor")
local monster = {}

monster.description = "Monstor"
monster.experience = 575
monster.outfit = {
	lookType = 244,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6336
monster.health = 960
monster.maxHealth = 960
monster.race = "blood"
monster.speed = 350
monster.manaCost = 0
monster.maxSummons = 3

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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "NO ARMY ME STOPPING! GRARR!", yell = false},
	{text = "ME DESTROY CITY! GROAR!", yell = false},
	{text = "WHARR! MUST ... KIDNAP WOMEN!", yell = false},
}

monster.loot = {
	{id = 10298, chance = 1000}, -- helmet of ultimate terror
	{id = 10303, chance = 1000}, -- farmer's avenger
	{id = 10297, chance = 1000}, -- shield of care
	{id = 10313, chance = 1000}, -- incredible mumpiz slayer
}

monster.attacks = {
	{name = "melee", interval = 1200, minDamage = 0, maxDamage = -167, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = -66, maxDamage = -85, effect = CONST_ME_GREENSHIMMER, target = false, length = 6, spread = 0, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 30, minDamage = 90, maxDamage = 200, effect = CONST_ME_FIRE, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = -7},
	{type = COMBAT_HOLYDAMAGE, percent = -3},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Acid Blob", chance = 40, interval = 4000, max = 3},
}

mType:register(monster)