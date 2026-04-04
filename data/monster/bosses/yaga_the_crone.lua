local mType = Game.createMonsterType("Yaga The Crone")
local monster = {}

monster.description = "Yaga The Crone"
monster.experience = 375
monster.outfit = {
	lookType = 54,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 620
monster.maxHealth = 620
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 0

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
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Where did I park my hut?", yell = false},
	{text = "You will taste so sweet!", yell = false},
	{text = "Hexipooh, bewitched are you!", yell = false},
}

monster.loot = {
	{id = 2654, chance = 66000}, -- cape
	{id = 2551, chance = 62500}, -- broom
	{id = 2687, chance = 62500, maxCount = 8}, -- cookie
	{id = 2148, chance = 29170, maxCount = 55}, -- gold coin
	{id = 2800, chance = 20833}, -- star herb
	{id = 2129, chance = 20833}, -- wolf tooth chain
	{id = 2199, chance = 8333}, -- garlic necklace
	{id = 8902, chance = 8333}, -- spellbook of mind control
	{id = 2651, chance = 4170}, -- coat
	{id = 2185, chance = 4170}, -- necrotic rod
	{id = 2402, chance = 4170}, -- silver dagger
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -50, interval = 2000, target = false},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -30, maxDamage = -50, interval = 2500, chance = 50, range = 5, target = true, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIRE},
	{name = "condition", type = CONDITION_POISON, interval = 3000, chance = 13, tick = 4000, minDamage = -10, maxDamage = -10, range = 5, shootEffect = CONST_ANI_POISON, target = true},
	{name = "firefield", interval = 2000, chance = 13, range = 5, target = true, shootEffect = CONST_ANI_FIRE},
}

monster.defenses = {
	defense = 20,
	armor = 15,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_REDSHIMMER},
	{name = "outfit", interval = 4000, chance = 9, effect = CONST_ME_REDSHIMMER, monster = "green frog", duration = 4000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_EARTHDAMAGE, percent = 1},
	{type = COMBAT_PHYSICALDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)