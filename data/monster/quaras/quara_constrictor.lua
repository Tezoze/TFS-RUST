local mType = Game.createMonsterType("Quara Constrictor")
local monster = {}

monster.description = "a quara constrictor"
monster.experience = 250
monster.outfit = {
	lookType = 46,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6065
monster.health = 450
monster.maxHealth = 450
monster.race = "blood"
monster.speed = 380
monster.manaCost = 670
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 30,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Gaaahhh!", yell = false},
	{text = "Gluh! Gluh!", yell = false},
	{text = "Tssss!", yell = false},
	{text = "Boohaa!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 87500, maxCount = 100}, -- gold coin
	{id = 2150, chance = 2860}, -- small amethyst
	{id = 2397, chance = 7761}, -- longsword
	{id = 2465, chance = 5000}, -- brass armor
	{id = 2670, chance = 5000, maxCount = 5}, -- shrimp
	{id = 5895, chance = 5940, maxCount = 2}, -- fish fin
	{id = 12443, chance = 14520}, -- quara tentacle
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false, condition = {type = CONDITION_POISON, startDamage = 20, interval = 2000}},
	{name = "combat", interval = 2000, chance = 10, minDamage = -50, maxDamage = -90, radius = 3, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -40, maxDamage = -70, range = 7, radius = 4, effect = CONST_ME_ICEATTACK, target = false, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, range = 1, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 14,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)