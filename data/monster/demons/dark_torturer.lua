local mType = Game.createMonsterType("Dark Torturer")
local monster = {}

monster.description = "a dark torturer"
monster.experience = 4650
monster.outfit = {
	lookType = 234,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6328
monster.health = 7350
monster.maxHealth = 7350
monster.race = "undead"
monster.speed = 318
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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You like it, don't you?", yell = false},
	{text = "IahaEhheAie!", yell = false},
	{text = "It's party time!", yell = false},
	{text = "Harrr, Harrr!", yell = false},
	{text = "The torturer is in!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 98250, maxCount = 256}, -- gold coin
	{id = 2671, chance = 65570, maxCount = 2}, -- ham
	{id = 6558, chance = 34180, maxCount = 3}, -- concentrated demonic blood
	{id = 5944, chance = 12080}, -- soul orb
	{id = 2152, chance = 10480, maxCount = 6}, -- platinum coin
	{id = 7591, chance = 9840}, -- great health potion
	{id = 6500, chance = 6630}, -- demonic essence
	{id = 2645, chance = 5350}, -- steel boots
	{id = 7368, chance = 2000, maxCount = 2}, -- assassin star
	{id = 6300, chance = 1960}, -- death ring
	{id = 2558, chance = 1430}, -- saw
	{id = 5022, chance = 930, maxCount = 2}, -- orichalcum pearl
	{id = 5480, chance = 860}, -- cat's paw
	{id = 5801, chance = 710}, -- jewelled backpack
	{id = 7412, chance = 680}, -- butcher's axe
	{id = 7388, chance = 320}, -- vile axe
	{id = 9971, chance = 70}, -- gold ingot
	{id = 2470, chance = 70}, -- golden legs
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -513, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -500, maxDamage = -700, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 45,
	{name = "combat", interval = 2000, chance = 10, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 90},
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)