local mType = Game.createMonsterType("Rahemos")
local monster = {}

monster.description = "Rahemos"
monster.experience = 3100
monster.outfit = {
	lookType = 87,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6031
monster.health = 3700
monster.maxHealth = 3700
monster.race = "undead"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 1

monster.changeTarget = {
	interval = 5000,
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
	{text = "It's a kind of magic.", yell = false},
	{text = "Abrah Kadabrah!", yell = false},
	{text = "Nothing hidden in my wrappings.", yell = false},
	{text = "It's not a trick, it's Rahemos.", yell = false},
	{text = "Meet my friend from hell!", yell = false},
	{text = "I will make you believe in magic.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 90}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 60}, -- gold coin
	{id = 2150, chance = 7000, maxCount = 3}, -- small amethyst
	{id = 2153, chance = 500}, -- violet gem
	{id = 2176, chance = 500}, -- orb
	{id = 2184, chance = 500}, -- crystal wand
	{id = 2214, chance = 7000}, -- ring of healing
	{id = 2348, chance = 100000}, -- ancient rune
	{id = 2447, chance = 200}, -- twin axe
	{id = 2662, chance = 300}, -- magician hat
	{id = 7590, chance = 7000}, -- great mana potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -750, target = false, condition = {type = CONDITION_POISON, startDamage = 65, interval = 2000}},
	{name = "combat", interval = 3000, chance = 7, minDamage = -75, maxDamage = -750, range = 1, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 20, minDamage = -60, maxDamage = -600, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 3000, chance = 20, minDamage = -60, maxDamage = -600, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 1000, chance = 12, radius = 6, effect = CONST_ME_POISON, target = false, speed = -650, duration = 60000},
	{name = "combat", interval = 1000, chance = 8, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGYAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "outfit", interval = 1000, chance = 15, range = 7, effect = CONST_ME_BLUESHIMMER, target = false},
}

monster.defenses = {
	defense = 35,
	armor = 30,
	{name = "combat", interval = 1000, chance = 20, minDamage = 200, maxDamage = 500, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "outfit", interval = 1000, chance = 5, effect = CONST_ME_BLUESHIMMER, monster = "demon", duration = 4000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 94},
	{type = COMBAT_ENERGYDAMAGE, percent = 92},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Demon", chance = 12, interval = 1000, max = 1},
}

mType:register(monster)