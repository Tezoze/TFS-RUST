local mType = Game.createMonsterType("Mutated Bat")
local monster = {}

monster.description = "a mutated bat"
monster.experience = 615
monster.outfit = {
	lookType = 307,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9829
monster.health = 900
monster.maxHealth = 900
monster.race = "blood"
monster.speed = 186
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 300,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Shriiiiiek", yell = false},
}

monster.loot = {
	{id = 2144, chance = 720, maxCount = 3}, -- black pearl
	{id = 2148, chance = 53000, maxCount = 50}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 70}, -- gold coin
	{id = 2150, chance = 500, maxCount = 2}, -- small amethyst
	{id = 2167, chance = 990}, -- energy ring
	{id = 2513, chance = 7760}, -- battle shield
	{id = 2529, chance = 70}, -- black shield
	{id = 2800, chance = 7260}, -- star herb
	{id = 2800, chance = 5060}, -- star herb
	{id = 5894, chance = 4900, maxCount = 2}, -- bat wing
	{id = 7386, chance = 110}, -- mercenary sword
	{id = 9808, chance = 12530},
	{id = 9809, chance = 12530, maxCount = 2},
	{id = 10016, chance = 80}, -- batwing hat
	{id = 10579, chance = 4900}, -- mutated bat ear
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -168, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -70, maxDamage = -180, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_POISON},
	{name = "combat", type = COMBAT_DROWNDAMAGE, minDamage = -30, maxDamage = -90, interval = 2000, chance = 15, radius = 6, target = false, effect = CONST_ME_WHITENOTE},
	{name = "mutated bat curse", interval = 2000, chance = 10, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -190, maxDamage = -240, length = 4, spread = 3, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 19,
	{name = "combat", interval = 2000, chance = 10, minDamage = 80, maxDamage = 95, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)