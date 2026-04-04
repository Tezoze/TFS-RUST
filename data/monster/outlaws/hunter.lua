local mType = Game.createMonsterType("Hunter")
local monster = {}

monster.description = "a hunter"
monster.experience = 150
monster.outfit = {
	lookType = 129,
	lookHead = 95,
	lookBody = 116,
	lookLegs = 121,
	lookFeet = 115,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 150
monster.maxHealth = 150
monster.race = "blood"
monster.speed = 210
monster.manaCost = 530
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
	{text = "Guess who we're hunting, hahaha!", yell = false},
	{text = "Guess who we are hunting!", yell = false},
	{text = "Bullseye!", yell = false},
	{text = "You'll make a nice trophy!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 3300}, -- torch
	{id = 2147, chance = 150}, -- small ruby
	{id = 2201, chance = 3000}, -- dragon necklace
	{id = 2456, chance = 5770}, -- bow
	{id = 2460, chance = 5050}, -- brass helmet
	{id = 2465, chance = 5070}, -- brass armor
	{id = 2544, chance = 82000, maxCount = 22}, -- arrow
	{id = 2545, chance = 4500, maxCount = 4}, -- poison arrow
	{id = 2546, chance = 5360, maxCount = 3}, -- burst arrow
	{id = 2675, chance = 20300, maxCount = 2}, -- orange
	{id = 2690, chance = 11370, maxCount = 2}, -- roll
	{id = 5875, chance = 1610}, -- sniper gloves
	{id = 5907, chance = 1120}, -- slingshot
	{id = 7394, chance = 1190}, -- wolf trophy
	{id = 7397, chance = 1520}, -- deer trophy
	{id = 7400, chance = 570}, -- lion trophy
	{id = 12425, chance = 10240}, -- hunter's quiver
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
	{name = "combat", interval = 2000, chance = 50, minDamage = 0, maxDamage = -100, range = 7, shootEffect = CONST_ANI_ARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}


mType:register(monster)