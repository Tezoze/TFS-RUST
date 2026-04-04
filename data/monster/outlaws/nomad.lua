local mType = Game.createMonsterType("Nomad")
local monster = {}

monster.description = "a nomad"
monster.experience = 60
monster.outfit = {
	lookType = 146,
	lookHead = 114,
	lookBody = 20,
	lookLegs = 22,
	lookFeet = 2,
	lookAddons = 2,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 160
monster.maxHealth = 160
monster.race = "blood"
monster.speed = 215
monster.manaCost = 420
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 18,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "We are the true sons of the desert!", yell = false},
	{text = "I will leave your remains to the vultures!", yell = false},
	{text = "We are swift as the wind of the desert!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 16250, maxCount = 51}, -- gold coin
	{id = 2666, chance = 22500, maxCount = 2}, -- meat
	{id = 2386, chance = 28000}, -- axe
	{id = 2511, chance = 20000}, -- brass shield
	{id = 2649, chance = 13750}, -- leather legs
	{id = 2398, chance = 12500}, -- mace
	{id = 2458, chance = 4500}, -- chain helmet
	{id = 1987, chance = 1}, -- bag
	{id = 2465, chance = 1500}, -- brass armor
	{id = 2509, chance = 750}, -- steel shield
	{id = 2459, chance = 450}, -- iron helmet
	{id = 8267, chance = 3200}, -- nomad parchment
	{id = 2419, chance = 3000}, -- scimitar
	{id = 8838, chance = 4840, maxCount = 3}, -- potato
	{id = 12412, chance = 2160}, -- dirty turban
	{id = 12448, chance = 6420}, -- rope belt
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 30, attack = 40, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -12},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)