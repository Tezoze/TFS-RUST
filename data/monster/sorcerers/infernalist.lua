local mType = Game.createMonsterType("Infernalist")
local monster = {}

monster.description = "an infernalist"
monster.experience = 4000
monster.outfit = {
	lookType = 130,
	lookHead = 78,
	lookBody = 76,
	lookLegs = 94,
	lookFeet = 115,
	lookAddons = 2,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 3650
monster.maxHealth = 3650
monster.race = "blood"
monster.speed = 230
monster.manaCost = 0
monster.maxSummons = 1

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
	targetDistance = 4,
	staticAttackChance = 95,
	runHealth = 900,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Nothing will remain but your scorched bones!", yell = false},
	{text = "Some like it hot!", yell = false},
	{text = "It's cooking time!", yell = false},
	{text = "Feel the heat of battle!", yell = false},
}

monster.loot = {
	{id = 1986, chance = 300}, -- red tome
	{id = 2114, chance = 220}, -- piggy bank
	{id = 2148, chance = 56500, maxCount = 100}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 47}, -- gold coin
	{id = 2167, chance = 1800}, -- energy ring
	{id = 2436, chance = 6500}, -- skull staff
	{id = 5904, chance = 600}, -- magic sulphur
	{id = 5911, chance = 1420}, -- red piece of cloth
	{id = 7590, chance = 19700}, -- great mana potion
	{id = 7591, chance = 1900}, -- great health potion
	{id = 7760, chance = 4250}, -- small enchanted ruby
	{id = 7891, chance = 300}, -- magma boots
	{id = 8840, chance = 8500, maxCount = 5}, -- raspberry
	{id = 8902, chance = 370}, -- spellbook of mind control
	{id = 9958, chance = 520}, -- royal tapestry
	{id = 9969, chance = 820}, -- black skull
	{id = 9971, chance = 70}, -- gold ingot
	{id = 9980, chance = 220}, -- crystal of power
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 40, minDamage = -65, maxDamage = -180, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -90, maxDamage = -180, range = 7, radius = 3, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -53, maxDamage = -120, range = 7, radius = 3, shootEffect = CONST_ANI_ENERGYBALL, effect = CONST_ME_TELEPORT, target = true, type = COMBAT_MANADRAIN},
	{name = "firefield", interval = 2000, chance = 15, range = 7, radius = 3, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 2000, chance = 10, minDamage = -150, maxDamage = -250, effect = CONST_ME_FIREATTACK, target = false, length = 8, spread = 0, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -100, maxDamage = -150, radius = 2, effect = CONST_ME_EXPLOSIONAREA, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 33,
	{name = "combat", interval = 2000, chance = 15, minDamage = 60, maxDamage = 230, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 95},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "fire elemental", chance = 20, interval = 2000, max = 1},
}

mType:register(monster)