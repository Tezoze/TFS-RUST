local mType = Game.createMonsterType("Pirate Corsair")
local monster = {}

monster.description = "a pirate corsair"
monster.experience = 350
monster.outfit = {
	lookType = 98,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 675
monster.maxHealth = 675
monster.race = "blood"
monster.speed = 238
monster.manaCost = 775
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 15
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 40,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Give up!", yell = false},
	{text = "Hiyaa!", yell = false},
	{text = "Plundeeeeer!", yell = false},
}

monster.loot = {
	{id = 2114, chance = 150}, -- piggy bank
	{id = 2148, chance = 50000, maxCount = 88}, -- gold coin
	{id = 2385, chance = 10000}, -- sabre
	{id = 2399, chance = 8400, maxCount = 12}, -- throwing star
	{id = 2489, chance = 1650}, -- dark armor
	{id = 2521, chance = 1000}, -- dark shield
	{id = 5462, chance = 220}, -- pirate boots
	{id = 5553, chance = 130}, -- rum flask
	{id = 5813, chance = 130}, -- skull candle
	{id = 5926, chance = 930}, -- pirate backpack
	{id = 6096, chance = 1150}, -- pirate hat
	{id = 6097, chance = 6000}, -- hook
	{id = 6098, chance = 5000}, -- eye patch
	{id = 6126, chance = 6000}, -- peg leg
	{id = 7588, chance = 820}, -- strong health potion
	{id = 11219, chance = 11050}, -- compass
	{id = 11219, chance = 11020}, -- compass
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -170, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -150, range = 7, shootEffect = CONST_ANI_THROWINGSTAR, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)