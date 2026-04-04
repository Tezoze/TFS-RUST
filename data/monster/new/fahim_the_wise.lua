local mType = Game.createMonsterType("Fahim the Wise")
local monster = {}

monster.description = "a fahim the wise"
monster.experience = 1500
monster.outfit = {
	lookType = 104,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6033
monster.health = 2000
monster.maxHealth = 2000
monster.race = "blood"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 3

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
	canPushCreatures = false,
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
	{text = "You should know better than to be an enemy of the Marid", yell = false},
}

monster.loot = {
	{id = 5912, chance = 99990, maxCount = 4}, -- blue piece of cloth
	{id = 12426, chance = 99990}, -- jewelled belt
	{id = 2148, chance = 95240, maxCount = 118}, -- gold coin
	{id = 12442, chance = 66670}, -- noble turban
	{id = 7378, chance = 57140, maxCount = 3}, -- royal spear
	{id = 11227, chance = 47620}, -- shiny stone
	{id = 7589, chance = 42860, maxCount = 3}, -- strong mana potion
	{id = 2677, chance = 40480, maxCount = 22}, -- blueberry
	{id = 2663, chance = 33330}, -- mystic turban
	{id = 2146, chance = 14290, maxCount = 2}, -- small sapphire
	{id = 7732, chance = 7140}, -- seeds
	{id = 7900, chance = 4760}, -- magma monocle
	{id = 2158, chance = 2380}, -- blue gem
	{id = 2063, chance = 580}, -- small oil lamp
	{id = 2070, chance = 480}, -- wooden flute
	{id = 2442, chance = 380}, -- heavy machete
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -130, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -300, range = 7, shootEffect = CONST_ANI_ENERGYBALL, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -90, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -650, duration = 1500},
	{name = "combat", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "outfit", interval = 2000, chance = 1, range = 7, effect = CONST_ME_BLUESHIMMER, target = false},
	{name = "combat", interval = 2000, chance = 15, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -90, radius = 3, effect = CONST_ME_ENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 20,
	{name = "combat", interval = 2000, chance = 15, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -1},
	{type = COMBAT_HOLYDAMAGE, percent = -1},
	{type = COMBAT_ICEDAMAGE, percent = 15},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "blue djinn", chance = 10, interval = 2000, max = 3},
}

mType:register(monster)