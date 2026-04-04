local mType = Game.createMonsterType("Dipthrah")
local monster = {}

monster.description = "Dipthrah"
monster.experience = 2900
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
monster.health = 4200
monster.maxHealth = 4200
monster.race = "undead"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 4

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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You can't escape death forever", yell = false},
	{text = "Come closer to learn the final lesson", yell = false},
	{text = "Undeath will shatter my shackles.", yell = false},
	{text = "You don't need this magic anymore.", yell = false},
}

monster.loot = {
	{id = 2146, chance = 7000, maxCount = 3}, -- small sapphire
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2158, chance = 1500}, -- blue gem
	{id = 2167, chance = 7000}, -- energy ring
	{id = 2178, chance = 1500}, -- mind stone
	{id = 2193, chance = 500}, -- ankh
	{id = 2354, chance = 100000}, -- ornamented ankh
	{id = 2436, chance = 500}, -- skull staff
	{id = 2446, chance = 300}, -- pharaoh sword
	{id = 7590, chance = 7000}, -- great mana potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false, condition = {type = CONDITION_POISON, startDamage = 65, interval = 2000}},
	{name = "combat", interval = 4000, chance = 20, minDamage = -100, maxDamage = -800, range = 1, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -500, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_MANADRAIN},
	{name = "speed", interval = 1000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -650, duration = 50000},
	{name = "combat", interval = 1000, chance = 12, radius = 7, effect = CONST_ME_BLUEBUBBLE, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "melee", interval = 3000, chance = 34, minDamage = -50, maxDamage = -600, radius = 3, effect = CONST_ME_BLUEBUBBLE, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 1000, chance = 25, minDamage = 100, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 30},
}

monster.immunities = {
	{type = "physical", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Priestess", chance = 15, interval = 2000, max = 3},
}

mType:register(monster)