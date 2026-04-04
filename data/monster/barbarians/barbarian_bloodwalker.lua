local mType = Game.createMonsterType("Barbarian Bloodwalker")
local monster = {}

monster.description = "a barbarian bloodwalker"
monster.experience = 195
monster.outfit = {
	lookType = 255,
	lookHead = 114,
	lookBody = 132,
	lookLegs = 132,
	lookFeet = 132,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 305
monster.maxHealth = 305
monster.race = "blood"
monster.speed = 236
monster.manaCost = 590
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "YAAAHEEE!", yell = false},
	{text = "SLAUGHTER!", yell = false},
	{text = "CARNAGE!", yell = false},
	{text = "You can run but you can't hide", yell = false},
}

monster.loot = {
	{id = 2044, chance = 8280}, -- lamp
	{id = 2148, chance = 55310, maxCount = 12}, -- gold coin
	{id = 2378, chance = 5910}, -- battle axe
	{id = 2381, chance = 6740}, -- halberd
	{id = 2458, chance = 10520}, -- chain helmet
	{id = 2464, chance = 10420}, -- chain armor
	{id = 2671, chance = 4900}, -- ham
	{id = 3962, chance = 380}, -- beastslayer axe
	{id = 5911, chance = 5540}, -- red piece of cloth
	{id = 7290, chance = 3000}, -- shard
	{id = 7457, chance = 100}, -- fur boots
	{id = 7618, chance = 980}, -- health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -240, target = false},
}

monster.defenses = {
	defense = 0,
	armor = 9,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 240, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = -5},
}


mType:register(monster)