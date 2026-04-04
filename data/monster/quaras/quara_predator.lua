local mType = Game.createMonsterType("Quara Predator")
local monster = {}

monster.description = "a quara predator"
monster.experience = 1600
monster.outfit = {
	lookType = 20,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6067
monster.health = 2200
monster.maxHealth = 2200
monster.race = "blood"
monster.speed = 450
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Rrrah!", yell = false},
	{text = "Rraaar!", yell = false},
	{text = "Gnarrr!", yell = false},
	{text = "Tcharrr!", yell = false},
}

monster.loot = {
	{id = 2145, chance = 5160, maxCount = 2}, -- small diamond
	{id = 2148, chance = 28000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 53}, -- gold coin
	{id = 2387, chance = 3171}, -- double axe
	{id = 2670, chance = 4860, maxCount = 5}, -- shrimp
	{id = 5741, chance = 400}, -- skull helmet
	{id = 5895, chance = 5940, maxCount = 2}, -- fish fin
	{id = 7368, chance = 590}, -- assassin star
	{id = 7378, chance = 9000, maxCount = 7}, -- royal spear
	{id = 7383, chance = 680}, -- relic sword
	{id = 7591, chance = 1000}, -- great health potion
	{id = 7897, chance = 420}, -- glacier robe
	{id = 12447, chance = 9090}, -- quara bone
	{id = 13305, chance = 20}, -- giant shrimp
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -473, target = false},
}

monster.defenses = {
	defense = 45,
	armor = 40,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_GREENSHIMMER, speed = 270, duration = 5000},
	{name = "combat", interval = 2000, chance = 10, minDamage = 25, maxDamage = 75, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)