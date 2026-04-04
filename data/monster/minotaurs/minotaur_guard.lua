local mType = Game.createMonsterType("Minotaur Guard")
local monster = {}

monster.description = "a minotaur guard"
monster.experience = 160
monster.outfit = {
	lookType = 29,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5983
monster.health = 185
monster.maxHealth = 185
monster.race = "blood"
monster.speed = 190
monster.manaCost = 550
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
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
	{text = "Kirrl Karrrl!", yell = false},
	{text = "Kaplar", yell = false},
}

monster.loot = {
	{id = 2148, chance = 59740, maxCount = 20}, -- gold coin
	{id = 2387, chance = 430}, -- double axe
	{id = 2464, chance = 3030}, -- chain armor
	{id = 2465, chance = 4130}, -- brass armor
	{id = 2513, chance = 2090}, -- battle shield
	{id = 2580, chance = 480}, -- fishing rod
	{id = 5878, chance = 940}, -- minotaur leather
	{id = 7401, chance = 100}, -- minotaur trophy
	{id = 7618, chance = 410}, -- health potion
	{id = 12428, chance = 8170, maxCount = 2}, -- minotaur horn
	{id = 12438, chance = 4990}, -- piece of warrior armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)