local mType = Game.createMonsterType("Annihilon")
local monster = {}

monster.description = "Annihilon"
monster.experience = 15000
monster.outfit = {
	lookType = 12,
	lookHead = 19,
	lookBody = 104,
	lookLegs = 96,
	lookFeet = 96,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6068
monster.health = 46500
monster.maxHealth = 46500
monster.race = "fire"
monster.speed = 132
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
	interval = 5000,
	chance = 10,
	{text = "Flee as long as you can!", yell = false},
	{text = "Annihilon's might will crush you all!", yell = false},
	{text = "I am coming for you!", yell = false},
}

monster.loot = {
	{id = 2127, chance = 20000}, -- emerald bangle
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 16666, maxCount = 30}, -- platinum coin
	{id = 2153, chance = 16666}, -- violet gem
	{id = 2154, chance = 20000}, -- yellow gem
	{id = 2155, chance = 12500}, -- green gem
	{id = 2156, chance = 20000}, -- red gem
	{id = 2158, chance = 20000}, -- blue gem
	{id = 2381, chance = 20000}, -- halberd
	{id = 2427, chance = 20000}, -- guardian halberd
	{id = 2452, chance = 25000}, -- heavy mace
	{id = 2514, chance = 4166}, -- mastermind shield
	{id = 2515, chance = 7692}, -- guardian shield
	{id = 2519, chance = 11111}, -- crown shield
	{id = 2520, chance = 4166}, -- demon shield
	{id = 2528, chance = 9090}, -- tower shield
	{id = 2547, chance = 16666, maxCount = 94}, -- power bolt
	{id = 5944, chance = 20000, maxCount = 5}, -- soul orb
	{id = 5954, chance = 12500, maxCount = 2}, -- demon horn
	{id = 6529, chance = 20000, maxCount = 46}, -- infernal bolt
	{id = 7366, chance = 16666, maxCount = 70}, -- viper star
	{id = 7368, chance = 16666, maxCount = 50}, -- assassin star
	{id = 7387, chance = 7142}, -- diamond sceptre
	{id = 7421, chance = 14285}, -- onyx flail
	{id = 7431, chance = 1234}, -- demonbone
	{id = 7439, chance = 16666}, -- berserk potion
	{id = 7440, chance = 14285}, -- mastermind potion
	{id = 7590, chance = 11111}, -- great mana potion
	{id = 7591, chance = 14285}, -- great health potion
	{id = 7632, chance = 33333, maxCount = 2},
	{id = 7840, chance = 20000, maxCount = 46}, -- flaming arrow
	{id = 8472, chance = 14285}, -- great spirit potion
	{id = 8473, chance = 14285}, -- ultimate health potion
	{id = 8877, chance = 1851}, -- lavos armor
	{id = 8891, chance = 10000}, -- paladin armor
	{id = 8928, chance = 1234}, -- obsidian truncheon
	{id = 9808, chance = 1234},
	{id = 9810, chance = 50000},
	{id = 9971, chance = 20000}, -- gold ingot
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -1707, target = false},
	{name = "combat", interval = 1000, chance = 11, minDamage = 0, maxDamage = -600, effect = CONST_ME_MORTAREA, target = false, length = 8, spread = 0, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -200, maxDamage = -700, radius = 4, effect = CONST_ME_ICEAREA, target = false, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 3000, chance = 18, minDamage = -50, maxDamage = -255, radius = 5, effect = CONST_ME_GROUNDSHAKER, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -50, maxDamage = -600, radius = 6, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 55,
	armor = 60,
	{name = "combat", interval = 1000, chance = 14, minDamage = 400, maxDamage = 900, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 4, effect = CONST_ME_BLUESHIMMER, speed = 500, duration = 7000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 96},
	{type = COMBAT_DEATHDAMAGE, percent = 95},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)