local mType = Game.createMonsterType("Zugurosh")
local monster = {}

monster.description = "Zugurosh"
monster.experience = 10000
monster.outfit = {
	lookType = 12,
	lookHead = 2,
	lookBody = 35,
	lookLegs = 57,
	lookFeet = 91,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8721
monster.health = 90500
monster.maxHealth = 90500
monster.race = "fire"
monster.speed = 330
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 15
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
	runHealth = 4500,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "You will run out of resources soon enough!", yell = true},
	{text = "One little mistake and you're all are mine!", yell = false},
	{text = "I sense your strength fading!", yell = false},
	{text = "I know you will show a weakness!", yell = false},
	{text = "Your fear will make you prone to mistakes!", yell = false},
}

monster.loot = {
	{id = 6500, chance = 100000}, -- demonic essence
	{id = 2148, chance = 100000, maxCount = 194}, -- gold coin
	{id = 9813, chance = 54000},
	{id = 9810, chance = 45000},
	{id = 7590, chance = 27000}, -- great mana potion
	{id = 8472, chance = 26000}, -- great spirit potion
	{id = 7591, chance = 23000}, -- great health potion
	{id = 8473, chance = 22000}, -- ultimate health potion
	{id = 9971, chance = 21000}, -- gold ingot
	{id = 2152, chance = 21000, maxCount = 30}, -- platinum coin
	{id = 6104, chance = 21000}, -- jewel case
	{id = 5944, chance = 21000, maxCount = 10}, -- soul orb
	{id = 2151, chance = 18000, maxCount = 30}, -- talon
	{id = 5911, chance = 17000, maxCount = 10}, -- red piece of cloth
	{id = 2134, chance = 17000}, -- silver brooch
	{id = 5912, chance = 15000, maxCount = 10}, -- blue piece of cloth
	{id = 5909, chance = 15000, maxCount = 10}, -- white piece of cloth
	{id = 5910, chance = 14000, maxCount = 10}, -- green piece of cloth
	{id = 5914, chance = 14000, maxCount = 10}, -- yellow piece of cloth
	{id = 5913, chance = 12000, maxCount = 10}, -- brown piece of cloth
	{id = 5954, chance = 9700, maxCount = 2}, -- demon horn
	{id = 2195, chance = 8700}, -- boots of haste
	{id = 2173, chance = 6000}, -- amulet of loss
	{id = 2645, chance = 4500}, -- steel boots
	{id = 2646, chance = 1500}, -- golden boots
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -800, interval = 2000, target = false},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = 0, maxDamage = -500, interval = 2000, chance = 10, range = 4, target = true, effect = CONST_ME_REDSHIMMER},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = 0, maxDamage = -500, interval = 2000, chance = 15, length = 7, spread = 0, target = false, effect = CONST_ME_MORTAREA},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = 0, maxDamage = -100, interval = 2000, chance = 15, radius = 4, target = false, effect = CONST_ME_SMALLCLOUDS},
	{name = "condition", type = CONDITION_FIRE, interval = 3000, chance = 20, tick = 10000, minDamage = -10, maxDamage = -10, radius = 4, effect = CONST_ME_EXPLOSIONAREA, target = true},
	{name = "combat", type = COMBAT_MANADRAIN, minDamage = -60, maxDamage = -200, interval = 1000, chance = 13, radius = 5, target = false, effect = CONST_ME_WATERSPLASH},
}

monster.defenses = {
	defense = 55,
	armor = 45,
	{name = "combat", interval = 2000, chance = 50, minDamage = 40, maxDamage = 60, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 2000, chance = 50, minDamage = 400, maxDamage = 600, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 1000, chance = 5, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 25},
	{type = COMBAT_EARTHDAMAGE, percent = 40},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)