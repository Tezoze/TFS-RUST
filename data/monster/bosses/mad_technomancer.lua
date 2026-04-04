local mType = Game.createMonsterType("Mad Technomancer")
local monster = {}

monster.description = "a mad technomancer"
monster.experience = 55
monster.outfit = {
	lookType = 66,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6015
monster.health = 1800
monster.maxHealth = 1800
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 500,
	chance = 25
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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 150,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I love the smell of firebombs in the morning.", yell = false},
	{text = "I'm going to make them an offer they can't refuse.", yell = false},
	{text = "My masterplan cannot fail!", yell = false},
	{text = "Gentlemen, you can't fight here! This is the War Room!", yell = false},
}

monster.loot = {
	{id = 7699, chance = 100000}, -- technomancer beard
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 50, attack = 40, target = false},
	{name = "combat", interval = 1000, chance = 10, minDamage = -50, maxDamage = -120, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 34, minDamage = -55, maxDamage = -105, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 25, minDamage = -50, maxDamage = -80, range = 7, target = false, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 1000, chance = 50, minDamage = 75, maxDamage = 325, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 80},
	{type = COMBAT_FIREDAMAGE, percent = 60},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)