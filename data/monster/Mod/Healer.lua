local mType = Game.createMonsterType("Healer")
local monster = {}

monster.description = "Healer"
monster.experience = 0
monster.outfit = {
	lookType = 9,
	lookHead = 20,
	lookBody = 30,
	lookLegs = 40,
	lookFeet = 50,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 2000
monster.maxHealth = 2000
monster.race = "blood"
monster.speed = 300
monster.manaCost = 1000
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 50
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 4000,
	chance = 300,
	{text = "can i heal u?", yell = false},
	{text = "are you injured?", yell = false},
}

monster.loot = {
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -240, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -120, range = 7, shootEffect = CONST_ANI_ARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 60,
	armor = 42,
	{name = "combat", interval = 1000, chance = 50, minDamage = 30, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 0},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)