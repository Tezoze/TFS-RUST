local mType = Game.createMonsterType("Lizard Sentinel")
local monster = {}

monster.description = "a lizard sentinel"
monster.experience = 110
monster.outfit = {
	lookType = 114,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6040
monster.health = 265
monster.maxHealth = 265
monster.race = "blood"
monster.speed = 180
monster.manaCost = 560
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Tssss!", yell = false},
}

monster.loot = {
	{id = 2145, chance = 190}, -- small diamond
	{id = 2148, chance = 89000, maxCount = 80}, -- gold coin
	{id = 2381, chance = 510}, -- halberd
	{id = 2389, chance = 8750, maxCount = 3}, -- spear
	{id = 2425, chance = 1120}, -- obsidian lance
	{id = 2464, chance = 8560}, -- chain armor
	{id = 2483, chance = 7730}, -- scale armor
	{id = 3965, chance = 4700}, -- hunting spear
	{id = 3974, chance = 320}, -- sentinel shield
	{id = 5876, chance = 2990}, -- lizard leather
	{id = 5881, chance = 3960}, -- lizard scale
	{id = 7618, chance = 590}, -- health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = 0, maxDamage = -70, range = 7, shootEffect = CONST_ANI_SPEAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 17,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)