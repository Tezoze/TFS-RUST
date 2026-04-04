local mType = Game.createMonsterType("Barbarian Brutetamer")
local monster = {}

monster.description = "a barbarian brutetamer"
monster.experience = 90
monster.outfit = {
	lookType = 264,
	lookHead = 78,
	lookBody = 116,
	lookLegs = 95,
	lookFeet = 121,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 145
monster.maxHealth = 145
monster.race = "blood"
monster.speed = 178
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 60000,
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
	staticAttackChance = 90,
	targetDistance = 4,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "To me, creatures of the wild!", yell = false},
	{text = "Feel the power of the beast!", yell = false},
	{text = "My instincts tell me about your cowardice.", yell = false},
}

monster.loot = {
	{id = 1958, chance = 4750},
	{id = 2148, chance = 90230, maxCount = 15}, -- gold coin
	{id = 2401, chance = 6550}, -- staff
	{id = 2464, chance = 9300}, -- chain armor
	{id = 2686, chance = 10940, maxCount = 2}, -- corncob
	{id = 3965, chance = 5200}, -- hunting spear
	{id = 7343, chance = 7590}, -- fur bag
	{id = 7379, chance = 340}, -- brutetamer's staff
	{id = 7457, chance = 170}, -- fur boots
	{id = 7463, chance = 150}, -- mammoth fur cape
	{id = 7464, chance = 90}, -- mammoth fur shorts
	{id = 7620, chance = 580}, -- mana potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -34, range = 7, radius = 1, shootEffect = CONST_ANI_SNOWBALL, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 8,
	{name = "combat", interval = 2000, chance = 40, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "War Wolf", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)