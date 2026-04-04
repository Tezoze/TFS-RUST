local mType = Game.createMonsterType("Betrayed Wraith")
local monster = {}

monster.description = "a betrayed wraith"
monster.experience = 3500
monster.outfit = {
	lookType = 233,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6316
monster.health = 4200
monster.maxHealth = 4200
monster.race = "undead"
monster.speed = 346
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 300,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Rrrah!", yell = false},
	{text = "Gnarr!", yell = false},
	{text = "Tcharrr!", yell = false},
}

monster.loot = {
	{id = 2145, chance = 11800, maxCount = 4}, -- small diamond
	{id = 2148, chance = 100000, maxCount = 200}, -- gold coin
	{id = 2152, chance = 100000, maxCount = 8}, -- platinum coin
	{id = 2547, chance = 50000, maxCount = 5}, -- power bolt
	{id = 5022, chance = 8000, maxCount = 2}, -- orichalcum pearl
	{id = 5741, chance = 390}, -- skull helmet
	{id = 5799, chance = 160}, -- golden figurine
	{id = 5944, chance = 10000}, -- soul orb
	{id = 6300, chance = 390}, -- death ring
	{id = 6500, chance = 19430}, -- demonic essence
	{id = 6558, chance = 65250}, -- concentrated demonic blood
	{id = 7368, chance = 10780, maxCount = 5}, -- assassin star
	{id = 7386, chance = 1890}, -- mercenary sword
	{id = 7416, chance = 80}, -- bloody edge
	{id = 7590, chance = 15000, maxCount = 3}, -- great mana potion
	{id = 8473, chance = 15410}, -- ultimate health potion
	{id = 11233, chance = 18410}, -- unholy bone
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -450, target = false},
	{name = "combat", chance = 10, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 2000, chance = 20, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, speed = -600, duration = 3000},
}

monster.defenses = {
	defense = 55,
	armor = 42,
	{name = "combat", interval = 2000, chance = 30, minDamage = 350, maxDamage = 600, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_REDSPARK},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 460, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)