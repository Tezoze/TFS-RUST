local mType = Game.createMonsterType("Pirate Buccaneer")
local monster = {}

monster.description = "a pirate buccaneer"
monster.experience = 250
monster.outfit = {
	lookType = 97,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 425
monster.maxHealth = 425
monster.race = "blood"
monster.speed = 218
monster.manaCost = 595
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
	runHealth = 50,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Give up!", yell = false},
	{text = "Hiyaa", yell = false},
	{text = "Plundeeeeer!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 10190}, -- torch
	{id = 2148, chance = 67740, maxCount = 59}, -- gold coin
	{id = 2238, chance = 9900}, -- worn leather boots
	{id = 2385, chance = 10100}, -- sabre
	{id = 2410, chance = 9000, maxCount = 5}, -- throwing knife
	{id = 2463, chance = 1130}, -- plate armor
	{id = 2513, chance = 3850}, -- battle shield
	{id = 5091, chance = 1000}, -- treasure map
	{id = 5553, chance = 120}, -- rum flask
	{id = 5792, chance = 40},
	{id = 5926, chance = 430}, -- pirate backpack
	{id = 6095, chance = 1200}, -- pirate shirt
	{id = 6097, chance = 4500}, -- hook
	{id = 6098, chance = 4200}, -- eye patch
	{id = 6126, chance = 5100}, -- peg leg
	{id = 7588, chance = 670}, -- strong health potion
	{id = 11219, chance = 9780}, -- compass
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -160, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -100, range = 4, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 16,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)