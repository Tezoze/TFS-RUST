local mType = Game.createMonsterType("Crazed Beggar")
local monster = {}

monster.description = "a crazed beggar"
monster.experience = 35
monster.outfit = {
	lookType = 153,
	lookHead = 59,
	lookBody = 38,
	lookLegs = 38,
	lookFeet = 97,
	lookAddons = 3,
	lookMount = 0
}

monster.corpse = 3058
monster.health = 100
monster.maxHealth = 100
monster.race = "blood"
monster.speed = 154
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
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hehehe!", yell = false},
	{text = "Raahhh!", yell = false},
	{text = "You are one of THEM! Die!", yell = false},
	{text = "Wanna buy roses??", yell = false},
	{text = "They're coming! They're coming!", yell = false},
	{text = "Make it stop!", yell = false},
	{text = "Gimme money!", yell = false},
}

monster.loot = {
	{id = 1681, chance = 420}, -- small blue pillow
	{id = 2072, chance = 360}, -- lute
	{id = 2148, chance = 99000, maxCount = 9}, -- gold coin
	{id = 2213, chance = 120}, -- dwarven ring
	{id = 2556, chance = 6500}, -- wooden hammer
	{id = 2567, chance = 9750}, -- wooden spoon
	{id = 2570, chance = 5650}, -- rolling pin
	{id = 2666, chance = 9500}, -- meat
	{id = 2690, chance = 22500}, -- roll
	{id = 2744, chance = 4700}, -- red rose
	{id = 2802, chance = 420}, -- sling herb
	{id = 5553, chance = 420}, -- rum flask
	{id = 6092, chance = 300}, -- very noble-looking watch
	{id = 9808, chance = 80},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -25, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 4,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)