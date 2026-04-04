local mType = Game.createMonsterType("Massive Energy Elemental")
local monster = {}

monster.description = "a massive energy elemental"
monster.experience = 950
monster.outfit = {
	lookType = 290,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8966
monster.health = 1100
monster.maxHealth = 1100
monster.race = "energy"
monster.speed = 430
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 15
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 85,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnPoison = false,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 91}, -- gold coin
	{id = 2150, chance = 3270, maxCount = 3}, -- small amethyst
	{id = 7589, chance = 17450}, -- strong mana potion
	{id = 7590, chance = 5450}, -- great mana potion
	{id = 7889, chance = 730}, -- lightning pendant
	{id = 7895, chance = 150}, -- lightning legs
	{id = 8901, chance = 360}, -- spellbook of warding
	{id = 8920, chance = 730}, -- wand of starstorm
	{id = 9809, chance = 730},
	{id = 10221, chance = 500}, -- shockwave amulet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -175, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -270, maxDamage = -615, range = 7, radius = 2, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -175, maxDamage = -205, range = 7, shootEffect = CONST_ANI_ENERGYBALL, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 20, effect = CONST_ME_YELLOWSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 35,
	{name = "combat", interval = 2000, chance = 5, minDamage = 190, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 70},
	{type = COMBAT_HOLYDAMAGE, percent = 25},
	{type = COMBAT_DEATHDAMAGE, percent = 1},
	{type = COMBAT_EARTHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)