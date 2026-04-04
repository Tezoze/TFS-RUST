local mType = Game.createMonsterType("Ron The Ripper")
local monster = {}

monster.description = "Ron The Ripper"
monster.experience = 500
monster.outfit = {
	lookType = 151,
	lookHead = 95,
	lookBody = 94,
	lookLegs = 117,
	lookFeet = 97,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 1500
monster.maxHealth = 1500
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
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
	staticAttackChance = 50,
	targetDistance = 1,
	runHealth = 250,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Muahaha!", yell = false},
}

monster.loot = {
	{id = 6101, chance = 100000}, -- ron the ripper's sabre
	{id = 2148, chance = 100000, maxCount = 128}, -- gold coin
	{id = 2229, chance = 81000, maxCount = 2}, -- skull
	{id = 2463, chance = 63000}, -- plate armor
	{id = 2379, chance = 45000}, -- dagger
	{id = 7591, chance = 18000}, -- great health potion
	{id = 2476, chance = 18000}, -- knight armor
	{id = 2666, chance = 18000}, -- meat
	{id = 5926, chance = 18000}, -- pirate backpack
	{id = 2145, chance = 9000}, -- small diamond
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
	{name = "combat", interval = 4000, chance = 60, minDamage = 0, maxDamage = -160, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 50,
	armor = 35,
	{name = "combat", interval = 4000, chance = 25, minDamage = 50, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)