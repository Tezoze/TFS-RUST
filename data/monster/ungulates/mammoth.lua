local mType = Game.createMonsterType("Mammoth")
local monster = {}

monster.description = "a mammoth"
monster.experience = 160
monster.outfit = {
	lookType = 199,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6074
monster.health = 320
monster.maxHealth = 320
monster.race = "blood"
monster.speed = 190
monster.manaCost = 500
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Troooooot!", yell = false},
	{text = "Hooooot-Toooooot!", yell = false},
	{text = "Tooooot.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 90000, maxCount = 40}, -- gold coin
	{id = 2666, chance = 39000}, -- meat
	{id = 2671, chance = 30000, maxCount = 3}, -- ham
	{id = 3973, chance = 500}, -- tusk shield
	{id = 7381, chance = 2800}, -- mammoth whopper
	{id = 7432, chance = 500}, -- furry club
	{id = 11224, chance = 7280}, -- thick fur
	{id = 11238, chance = 7500, maxCount = 2}, -- mammoth tusk
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}


mType:register(monster)