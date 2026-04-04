local mType = Game.createMonsterType("Mutated Human")
local monster = {}

monster.description = "a mutated human"
monster.experience = 150
monster.outfit = {
	lookType = 323,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9107
monster.health = 240
monster.maxHealth = 240
monster.race = "blood"
monster.speed = 154
monster.manaCost = 0
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
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Take that creature off my back!! I can feel it!", yell = false},
	{text = "You will regret interrupting my studies!", yell = false},
	{text = "You will be the next infected one... CRAAAHHH!", yell = false},
	{text = "Science... is a curse.", yell = false},
	{text = "Run as fast as you can.", yell = false},
	{text = "Oh by the gods! What is this... aaaaaargh!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 44000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 44000, maxCount = 50}, -- gold coin
	{id = 2161, chance = 4980}, -- strange talisman
	{id = 2170, chance = 70}, -- silver amulet
	{id = 2226, chance = 10050}, -- fishbone
	{id = 2376, chance = 5030}, -- sword
	{id = 2483, chance = 8080}, -- scale armor
	{id = 2696, chance = 8000}, -- cheese
	{id = 2801, chance = 420}, -- fern
	{id = 3976, chance = 7110, maxCount = 2}, -- worm
	{id = 7910, chance = 580}, -- peanut
	{id = 9808, chance = 190},
	{id = 11225, chance = 19940}, -- mutated flesh
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -90, interval = 2000, target = false},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = -50, maxDamage = -60, interval = 2000, chance = 15, length = 3, spread = 1, target = false, effect = CONST_ME_POISON},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 20, tick = 4000, minDamage = -190, maxDamage = -280, range = 1, effect = CONST_ME_HITBYPOISON, target = true},
	{name = "speed", interval = 2000, chance = 10, range = 7, target = true, effect = CONST_ME_STUN, speed = -600, duration = 30000},
}

monster.defenses = {
	defense = 15,
	armor = 14,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_GREENBUBBLE, speed = 220, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)