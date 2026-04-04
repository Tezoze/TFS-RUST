local mType = Game.createMonsterType("Lizard Dragon Priest")
local monster = {}

monster.description = "a lizard dragon priest"
monster.experience = 1188
monster.outfit = {
	lookType = 339,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11280
monster.health = 1450
monster.maxHealth = 1450
monster.race = "blood"
monster.speed = 256
monster.manaCost = 0
monster.maxSummons = 2

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
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 50,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "I ssssmell warm blood!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 93890, maxCount = 190}, -- gold coin
	{id = 7589, chance = 12080}, -- strong mana potion
	{id = 11361, chance = 9960}, -- dragon priest's wandtip
	{id = 7590, chance = 8030}, -- great mana potion
	{id = 2150, chance = 4870, maxCount = 3}, -- small amethyst
	{id = 2152, chance = 4040, maxCount = 2}, -- platinum coin
	{id = 2187, chance = 1530}, -- wand of inferno
	{id = 5876, chance = 1060}, -- lizard leather
	{id = 5881, chance = 1060}, -- lizard scale
	{id = 2181, chance = 1000}, -- terra rod
	{id = 2154, chance = 960}, -- yellow gem
	{id = 11245, chance = 950}, -- bunch of ripe rice
	{id = 2168, chance = 780}, -- life ring
	{id = 8871, chance = 670}, -- focus cape
	{id = 11303, chance = 430}, -- Zaoan shoes
	{id = 11356, chance = 290}, -- Zaoan robe
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -50, interval = 2000, target = false},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -125, maxDamage = -190, interval = 2000, chance = 20, range = 7, target = true, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREATTACK},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -320, maxDamage = -400, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 22,
	{name = "combat", interval = 2000, chance = 30, minDamage = 200, maxDamage = 300, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 45},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Dragon Hatchling", chance = 20, interval = 2000, max = 2},
}

mType:register(monster)