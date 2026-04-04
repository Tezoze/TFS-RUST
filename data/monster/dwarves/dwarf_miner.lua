local mType = Game.createMonsterType("Dwarf Miner")
local monster = {}

monster.description = "a dwarf miner"
monster.experience = 60
monster.outfit = {
	lookType = 160,
	lookHead = 77,
	lookBody = 101,
	lookLegs = 97,
	lookFeet = 115,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6007
monster.health = 120
monster.maxHealth = 120
monster.race = "blood"
monster.speed = 170
monster.manaCost = 420
monster.maxSummons = 0

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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Work, work!", yell = false},
	{text = "Intruders in the mines!", yell = false},
	{text = "Mine, all mine!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 33333, maxCount = 10}, -- gold coin
	{id = 2213, chance = 793}, -- dwarven ring
	{id = 2386, chance = 14285}, -- axe
	{id = 2484, chance = 6666}, -- studded armor
	{id = 2553, chance = 11111}, -- pick
	{id = 2649, chance = 9090}, -- leather legs
	{id = 2666, chance = 3846}, -- meat
	{id = 5880, chance = 3793}, -- iron ore
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -26, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 7,
}


mType:register(monster)