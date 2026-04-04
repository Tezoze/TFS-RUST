local mType = Game.createMonsterType("Latrivan")
local monster = {}

monster.description = "Latrivan"
monster.experience = 10000
monster.outfit = {
	lookType = 12,
	lookHead = 120,
	lookBody = 128,
	lookLegs = 121,
	lookFeet = 111,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8721
monster.health = 25000
monster.maxHealth = 25000
monster.race = "fire"
monster.speed = 390
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I might reward you for killing my brother ~ with a swift death!", yell = true},
	{text = "Colateral damage is so fun!", yell = false},
	{text = "Golgordan you fool!", yell = false},
	{text = "We are the brothers of fear!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 273}, -- gold coin
	{id = 7591, chance = 55000}, -- great health potion
	{id = 2387, chance = 30000}, -- double axe
	{id = 6300, chance = 25000}, -- death ring
	{id = 2214, chance = 25000}, -- ring of healing
	{id = 2144, chance = 20000, maxCount = 13}, -- black pearl
	{id = 2149, chance = 20000, maxCount = 10}, -- small emerald
	{id = 2396, chance = 15000}, -- ice rapier
	{id = 2162, chance = 15000}, -- magic light wand
	{id = 2170, chance = 15000}, -- silver amulet
	{id = 2146, chance = 15000, maxCount = 10}, -- small sapphire
	{id = 2143, chance = 15000, maxCount = 13}, -- white pearl
	{id = 2520, chance = 10000}, -- demon shield
	{id = 6500, chance = 10000}, -- demonic essence
	{id = 2167, chance = 10000}, -- energy ring
	{id = 2393, chance = 10000}, -- giant sword
	{id = 9971, chance = 10000}, -- gold ingot
	{id = 2179, chance = 10000}, -- gold ring
	{id = 2470, chance = 10000}, -- golden legs
	{id = 2158, chance = 5000}, -- blue gem
	{id = 2462, chance = 5000}, -- devil helmet
	{id = 2432, chance = 5000}, -- fire axe
	{id = 2155, chance = 5000}, -- green gem
	{id = 2164, chance = 5000}, -- might ring
	{id = 2402, chance = 5000}, -- silver dagger
	{id = 2150, chance = 15000, maxCount = 12}, -- small amethyst
	{id = 2182, chance = 5000}, -- snakebite rod
	{id = 2165, chance = 5000}, -- stealth ring
	{id = 2197, chance = 5000}, -- stone skin amulet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -1, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -850, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 10, minDamage = -50, maxDamage = -250, effect = CONST_ME_EXPLOSION, target = false, length = 7, spread = 0, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -600, range = 4, radius = 1, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -200, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 35,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -1},
	{type = COMBAT_ENERGYDAMAGE, percent = 1},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)