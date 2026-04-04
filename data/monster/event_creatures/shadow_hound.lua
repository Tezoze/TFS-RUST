local mType = Game.createMonsterType("Shadow Hound")
local monster = {}

monster.description = "a shadow hound"
monster.experience = 600
monster.outfit = {
	lookType = 322,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9923
monster.health = 555
monster.maxHealth = 555
monster.race = "blood"
monster.speed = 230
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 0,
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrr!", yell = true},
}

monster.loot = {
	{id = 10531, chance = 8333}, -- midnight shard
}

monster.attacks = {
	{name = "melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -350, target = false},
	{name = "combat", interval = 2000, chance = 24, minDamage = -60, maxDamage = -160, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREATTACK, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 55,
	armor = 38,
	{name = "combat", interval = 1000, chance = 15, minDamage = 60, maxDamage = 230, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)