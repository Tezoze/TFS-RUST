local mType = Game.createMonsterType("Energy Elemental")
local monster = {}

monster.description = "an energy elemental"
monster.experience = 550
monster.outfit = {
	lookType = 293,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8966
monster.health = 500
monster.maxHealth = 500
monster.race = "energy"
monster.speed = 230
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
	canPushItems = false,
	staticAttackChance = 85,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnPoison = false,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
}

monster.loot = {
	{id = 2124, chance = 2000}, -- crystal ring
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 70}, -- gold coin
	{id = 2150, chance = 5000, maxCount = 2}, -- small amethyst
	{id = 2167, chance = 892}, -- energy ring
	{id = 2170, chance = 1020}, -- silver amulet
	{id = 2189, chance = 636}, -- wand of cosmic energy
	{id = 2399, chance = 9900, maxCount = 5}, -- throwing star
	{id = 2425, chance = 3571}, -- obsidian lance
	{id = 2515, chance = 243}, -- guardian shield
	{id = 7449, chance = 5882}, -- crystal sword
	{id = 7589, chance = 7692}, -- strong mana potion
	{id = 7620, chance = 11711}, -- mana potion
	{id = 7838, chance = 10000, maxCount = 10}, -- flash arrow
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -175, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -125, maxDamage = -252, range = 7, radius = 2, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -130, range = 7, shootEffect = CONST_ANI_ENERGYBALL, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 20, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 25,
	{name = "combat", interval = 2000, chance = 10, minDamage = 90, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_EARTHDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)