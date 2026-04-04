local mType = Game.createMonsterType("Minotaur Archer")
local monster = {}

monster.description = "a minotaur archer"
monster.experience = 65
monster.outfit = {
	lookType = 24,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5982
monster.health = 100
monster.maxHealth = 100
monster.race = "blood"
monster.speed = 160
monster.manaCost = 390
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
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ruan Wihmpy!", yell = false},
	{text = "Kaplar!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 59740, maxCount = 20}, -- gold coin
	{id = 12428, chance = 8170, maxCount = 2}, -- minotaur horn
	{id = 12438, chance = 4990}, -- piece of warrior armor
	{id = 2465, chance = 4130}, -- brass armor
	{id = 2464, chance = 3030}, -- chain armor
	{id = 2513, chance = 2090}, -- battle shield
	{id = 5878, chance = 940}, -- minotaur leather
	{id = 2580, chance = 480}, -- fishing rod
	{id = 2387, chance = 430}, -- double axe
	{id = 7618, chance = 410}, -- health potion
	{id = 7401, chance = 100}, -- minotaur trophy
}

monster.attacks = {
	{name = "combat", interval = 2000, chance = 40, minDamage = 0, maxDamage = -100, range = 7, shootEffect = CONST_ANI_BOLT, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 6,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)