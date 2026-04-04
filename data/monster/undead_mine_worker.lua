local mType = Game.createMonsterType("Undead Mine Worker")
local monster = {}

monster.description = "an undead mine worker"
monster.experience = 45
monster.outfit = {
	lookType = 33,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 20449
monster.health = 100
monster.maxHealth = 100
monster.race = "undead"
monster.speed = 170
monster.manaCost = 300
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 8,
	{text = "Work... must work...", yell = false},
	{text = "No rest for the dead...", yell = false},
}

monster.loot = {
	{id = 2148, chance = 30000, maxCount = 10}, -- gold coin
	{id = 2553, chance = 15000}, -- pick
	{id = 2554, chance = 10000}, -- shovel
	{id = 5880, chance = 20000}, -- iron ore
	{id = 13757, chance = 10000}, -- coal
	{id = 11208, chance = 5000}, -- rotten piece of cloth
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)