local mType = Game.createMonsterType("Hellfire Fighter")
local monster = {}

monster.description = "a hellfire fighter"
monster.experience = 3120
monster.outfit = {
	lookType = 243,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6324
monster.health = 3800
monster.maxHealth = 3800
monster.race = "fire"
monster.speed = 330
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
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.loot = {
	{id = 2127, chance = 2200}, -- emerald bangle
	{id = 2136, chance = 190}, -- demonbone amulet
	{id = 2145, chance = 1400}, -- small diamond
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 10000, maxCount = 46}, -- gold coin
	{id = 2187, chance = 9450}, -- wand of inferno
	{id = 2239, chance = 50000}, -- burnt scroll
	{id = 2260, chance = 30000, maxCount = 2}, -- blank rune
	{id = 2392, chance = 4140}, -- fire sword
	{id = 2432, chance = 440}, -- fire axe
	{id = 5944, chance = 12150}, -- soul orb
	{id = 6500, chance = 14500}, -- demonic essence
	{id = 7894, chance = 730}, -- magma legs
	{id = 7899, chance = 470}, -- magma coat
	{id = 10553, chance = 9570}, -- fiery heart
	{id = 10581, chance = 5060}, -- piece of hellfire armor
	{id = 8748, chance = 670},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -520, target = false},
	{name = "firefield", interval = 2000, chance = 10, range = 7, radius = 3, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 2000, chance = 10, minDamage = -392, maxDamage = -1500, effect = CONST_ME_FIREATTACK, target = false, length = 8, spread = 0, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -330, range = 7, radius = 3, effect = CONST_ME_FIRE, target = false, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 62,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 25},
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)