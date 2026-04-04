local mType = Game.createMonsterType("Zoralurk")
local monster = {}

monster.description = "Zoralurk"
monster.experience = 30000
monster.outfit = {
	lookType = 12,
	lookHead = 0,
	lookBody = 98,
	lookLegs = 86,
	lookFeet = 94,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6068
monster.health = 55000
monster.maxHealth = 55000
monster.race = "undead"
monster.speed = 400
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 10000,
	chance = 20
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
	staticAttackChance = 98,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 30,
	{text = "I AM ZORALURK, THE DEMON WITH A THOUSAND FACES!", yell = true},
	{text = "BRING IT, COCKROACHES!", yell = true},
}

monster.loot = {
	{id = 2143, chance = 10000, maxCount = 5}, -- white pearl
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 90}, -- gold coin
	{id = 2195, chance = 16033}, -- boots of haste
	{id = 2393, chance = 60000}, -- giant sword
	{id = 2407, chance = 20000}, -- bright sword
	{id = 2407, chance = 20000}, -- bright sword
	{id = 2408, chance = 6000}, -- warlord sword
	{id = 2641, chance = 7000}, -- patched boots
	{id = 6530, chance = 16000}, -- worn leather boots
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -1013, target = false},
	{name = "combat", interval = 1000, chance = 12, minDamage = -600, maxDamage = -900, radius = 7, effect = CONST_ME_ENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 12, minDamage = -400, maxDamage = -800, radius = 7, effect = CONST_ME_SMALLPLANTS, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 25, minDamage = -500, maxDamage = -800, range = 7, effect = CONST_ME_BLUESHIMMER, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 3000, chance = 35, minDamage = -200, maxDamage = -600, range = 7, radius = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 65,
	armor = 55,
	{name = "combat", interval = 2000, chance = 35, minDamage = 300, maxDamage = 800, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 4000, chance = 80, effect = CONST_ME_REDSHIMMER, speed = 440, duration = 6000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_DICE, monster = "behemoth", duration = 10000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_DICE, monster = "fire devil", duration = 10000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_DICE, monster = "giant spider", duration = 10000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_DICE, monster = "undead dragon", duration = 10000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_DICE, monster = "lost soul", duration = 10000},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "demon", chance = 50, interval = 4000, max = 2},
}

mType:register(monster)