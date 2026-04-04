local mType = Game.createMonsterType("Gravelord Oshuran")
local monster = {}

monster.description = "Gravelord Oshuran"
monster.experience = 2400
monster.outfit = {
	lookType = 99,
	lookHead = 95,
	lookBody = 116,
	lookLegs = 119,
	lookFeet = 115,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6028
monster.health = 3100
monster.maxHealth = 3100
monster.race = "undead"
monster.speed = 220
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 4000,
	chance = 10
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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Your mortality is disgusting!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 115}, -- gold coin
	{id = 7589, chance = 17500}, -- strong mana potion
	{id = 2144, chance = 15960}, -- black pearl
	{id = 2143, chance = 15000}, -- white pearl
	{id = 2214, chance = 15040}, -- ring of healing
	{id = 2656, chance = 500}, -- blue robe
	{id = 7893, chance = 900}, -- lightning boots
	{id = 8904, chance = 300}, -- spellscroll of prophecies
	{id = 2175, chance = 4650}, -- spellbook
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
	{name = "speed", interval = 2000, chance = 25, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -300, duration = 30000},
	{name = "combat", interval = 2000, chance = 10, minDamage = -180, maxDamage = -300, effect = CONST_ME_REDSHIMMER, target = false, length = 7, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -350, effect = CONST_ME_GREENSPARK, target = false, length = 7, spread = 0, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -200, maxDamage = -245, range = 1, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 3000, chance = 15, minDamage = 100, maxDamage = 120, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 35},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 80},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Bonebeast", chance = 10, interval = 2000, max = 4},
}

mType:register(monster)