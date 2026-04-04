local mType = Game.createMonsterType("Bandit")
local monster = {}

monster.description = "a bandit"
monster.experience = 65
monster.outfit = {
	lookType = 129,
	lookHead = 58,
	lookBody = 59,
	lookLegs = 45,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 245
monster.maxHealth = 245
monster.race = "blood"
monster.speed = 180
monster.manaCost = 450
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 25,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Your money or your life!", yell = false},
	{text = "Hand me your purse!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 49000, maxCount = 30}, -- gold coin
	{id = 2386, chance = 29900}, -- axe
	{id = 2398, chance = 10100}, -- mace
	{id = 2458, chance = 5000}, -- chain helmet
	{id = 2459, chance = 520}, -- iron helmet
	{id = 2465, chance = 2500}, -- brass armor
	{id = 2511, chance = 16800}, -- brass shield
	{id = 2649, chance = 15500}, -- leather legs
	{id = 2685, chance = 7630, maxCount = 2}, -- tomato
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 11,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)