local mType = Game.createMonsterType("Dwarf Geomancer")
local monster = {}

monster.description = "a dwarf geomancer"
monster.experience = 265
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
monster.health = 380
monster.maxHealth = 380
monster.race = "blood"
monster.speed = 200
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
	targetDistance = 4,
	staticAttackChance = 70,
	runHealth = 110,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hail Durin!", yell = false},
	{text = "Earth is the strongest element.", yell = false},
	{text = "Dust to dust.", yell = false},
}

monster.loot = {
	{id = 2146, chance = 710}, -- small sapphire
	{id = 2148, chance = 50000, maxCount = 35}, -- gold coin
	{id = 2162, chance = 14000}, -- magic light wand
	{id = 2175, chance = 360}, -- spellbook
	{id = 2213, chance = 530}, -- dwarven ring
	{id = 2260, chance = 33000}, -- blank rune
	{id = 2423, chance = 1120}, -- clerical mace
	{id = 2673, chance = 25000}, -- pear
	{id = 2787, chance = 60000, maxCount = 2}, -- white mushroom
	{id = 5880, chance = 3120}, -- iron ore
	{id = 7886, chance = 470}, -- terra boots
	{id = 12414, chance = 8000}, -- geomancer's robe
	{id = 12419, chance = 7000}, -- geomancer's staff
}

monster.attacks = {
	{name = "combat", interval = 2000, chance = 20, minDamage = -50, maxDamage = -110, range = 7, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_GREENBUBBLE, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -50, maxDamage = -80, range = 7, target = false, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 40, minDamage = 75, maxDamage = 125, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 60},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)