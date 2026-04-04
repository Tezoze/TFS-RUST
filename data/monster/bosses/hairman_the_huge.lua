local mType = Game.createMonsterType("Hairman The Huge")
local monster = {}

monster.description = "Hairman The Huge"
monster.experience = 335
monster.outfit = {
	lookType = 116,
	lookHead = 20,
	lookBody = 30,
	lookLegs = 40,
	lookFeet = 50,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6043
monster.health = 600
monster.maxHealth = 600
monster.race = "blood"
monster.speed = 230
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 14
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 60}, -- gold coin
	{id = 5883, chance = 2500}, -- ape fur
	{id = 2676, chance = 7500, maxCount = 2}, -- banana
	{id = 2200, chance = 3000}, -- protection amulet
	{id = 2166, chance = 7500}, -- power ring
	{id = 2463, chance = 5000}, -- plate armor
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 45, attack = 40, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 20,
	{name = "speed", interval = 1000, chance = 7, effect = CONST_ME_REDSHIMMER, speed = 260, duration = 3000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 5},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)