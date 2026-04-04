local mType = Game.createMonsterType("Death Blob")
local monster = {}

monster.description = "a death blob"
monster.experience = 300
monster.outfit = {
	lookType = 315,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9960
monster.health = 320
monster.maxHealth = 320
monster.race = "undead"
monster.speed = 160
monster.manaCost = 0
monster.maxSummons = 3

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
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnPoison = true,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Shlorz", yell = false},
}

monster.loot = {
	{id = 9968, chance = 18470}, -- glob of tar
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = -35, maxDamage = -60, range = 3, radius = 4, effect = CONST_ME_POFF, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 5, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 5, minDamage = 20, maxDamage = 30, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}

monster.summons = {
	{name = "Death Blob", chance = 10, interval = 2000, max = 3},
}

mType:register(monster)