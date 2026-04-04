local mType = Game.createMonsterType("Quara Constrictor Scout")
local monster = {}

monster.description = "a quara constrictor scout"
monster.experience = 200
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
monster.speed = 150
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
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 20,
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
	{id = 2148, chance = 98800, maxCount = 49}, -- gold coin
	{id = 2150, chance = 4350}, -- small amethyst
	{id = 2397, chance = 8310}, -- longsword
	{id = 2465, chance = 4660}, -- brass armor
	{id = 2670, chance = 9680, maxCount = 3}, -- shrimp
	{id = 5895, chance = 5940, maxCount = 2}, -- fish fin
	{id = 12443, chance = 15600}, -- quara tentacle
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -135, target = false},
	{name = "combat", interval = 2000, chance = 15, maxDamage = -80, radius = 3, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 14,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)