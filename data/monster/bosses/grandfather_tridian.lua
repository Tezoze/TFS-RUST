local mType = Game.createMonsterType("Grandfather Tridian")
local monster = {}

monster.description = "Grandfather Tridian"
monster.experience = 1400
monster.outfit = {
	lookType = 193,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 1800
monster.maxHealth = 1800
monster.race = "blood"
monster.speed = 210
monster.manaCost = 0
monster.maxSummons = 2

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
	staticAttackChance = 50,
	targetDistance = 4,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I will bring peace to your misguided soul!", yell = false},
	{text = "Your intrusion can't be tolerated!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 80}, -- gold coin
	{id = 2114, chance = 100000}, -- piggy bank
	{id = 7589, chance = 5000}, -- strong mana potion
	{id = 2789, chance = 5000}, -- brown mushroom
	{id = 2187, chance = 5000}, -- wand of inferno
	{id = 2436, chance = 5000}, -- skull staff
	{id = 8922, chance = 5000}, -- wand of voodoo
	{id = 7426, chance = 3000}, -- amber staff
	{id = 6087, chance = 3000}, -- music sheet
	{id = 6088, chance = 3000}, -- music sheet
	{id = 6089, chance = 3000}, -- music sheet
	{id = 6090, chance = 3000}, -- music sheet
	{id = 3955, chance = 1000}, -- voodoo doll
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = -138, maxDamage = -362, range = 1, radius = 1, shootEffect = CONST_ANI_HOLY, effect = CONST_ME_HOLYAREA, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -50, range = 1, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 2000, chance = 25, minDamage = 60, maxDamage = 90, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_YELLOWBUBBLE},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 5},
	{type = COMBAT_PHYSICALDAMAGE, percent = 35},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Crypt Shambler", chance = 10, interval = 2000, max = 2},
	{name = "Ghost", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)