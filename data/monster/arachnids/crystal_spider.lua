local mType = Game.createMonsterType("Crystal Spider")
local monster = {}

monster.description = "a crystal spider"
monster.experience = 900
monster.outfit = {
	lookType = 263,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7344
monster.health = 1250
monster.maxHealth = 1250
monster.race = "venom"
monster.speed = 230
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 15
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Screeech!", yell = false},
}

monster.loot = {
	{id = 2124, chance = 1560}, -- crystal ring
	{id = 2125, chance = 1640}, -- crystal necklace
	{id = 2148, chance = 99998, maxCount = 100}, -- gold coin
	{id = 2148, chance = 99998, maxCount = 92}, -- gold coin
	{id = 2169, chance = 1480}, -- time ring
	{id = 2171, chance = 130}, -- platinum amulet
	{id = 2457, chance = 5200}, -- steel helmet
	{id = 2463, chance = 9993}, -- plate armor
	{id = 2476, chance = 560}, -- knight armor
	{id = 2477, chance = 760}, -- knight legs
	{id = 5801, chance = 80}, -- jewelled backpack
	{id = 5879, chance = 2010}, -- spider silk
	{id = 7290, chance = 7400}, -- shard
	{id = 7364, chance = 5840, maxCount = 6}, -- sniper arrow
	{id = 7437, chance = 140}, -- sapphire hammer
	{id = 7449, chance = 2490}, -- crystal sword
	{id = 7589, chance = 14950}, -- strong mana potion
	{id = 7902, chance = 670}, -- glacier mask
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false, condition = {type = CONDITION_POISON, startDamage = 160, interval = 2000}},
	{name = "speed", interval = 2000, chance = 15, range = 7, radius = 6, effect = CONST_ME_POFF, target = false, speed = -800, duration = 15000},
	{name = "combat", interval = 2000, chance = 15, minDamage = -50, maxDamage = -100, range = 7, shootEffect = CONST_ANI_ICE, effect = CONST_ME_ICEAREA, target = true, type = COMBAT_ICEDAMAGE},
	{name = "speed", interval = 2000, chance = 20, range = 7, shootEffect = CONST_ANI_SNOWBALL, target = true, speed = -600, duration = 10000},
}

monster.defenses = {
	defense = 0,
	armor = 43,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 250, duration = 5000},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)