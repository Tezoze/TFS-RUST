local mType = Game.createMonsterType("Dwarf")
local monster = {}

monster.description = "a dwarf"
monster.experience = 45
monster.outfit = {
	lookType = 69,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6007
monster.health = 90
monster.maxHealth = 90
monster.race = "blood"
monster.speed = 170
monster.manaCost = 320
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
	chance = 10,
	{text = "Hail Durin!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 35000, maxCount = 8}, -- gold coin
	{id = 2213, chance = 100}, -- dwarven ring
	{id = 2386, chance = 15000}, -- axe
	{id = 2388, chance = 25000}, -- hatchet
	{id = 2484, chance = 8000}, -- studded armor
	{id = 2530, chance = 10000}, -- copper shield
	{id = 2553, chance = 10000}, -- pick
	{id = 2597, chance = 8000}, -- letter
	{id = 2649, chance = 10000}, -- leather legs
	{id = 2787, chance = 50000}, -- white mushroom
	{id = 5880, chance = 3700}, -- iron ore
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)