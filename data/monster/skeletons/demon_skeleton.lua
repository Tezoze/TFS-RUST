local mType = Game.createMonsterType("Demon Skeleton")
local monster = {}

monster.description = "a demon skeleton"
monster.experience = 240
monster.outfit = {
	lookType = 37,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5963
monster.health = 400
monster.maxHealth = 400
monster.race = "undead"
monster.speed = 180
monster.manaCost = 620
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
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2050, chance = 5270}, -- torch
	{id = 2144, chance = 2900}, -- black pearl
	{id = 2147, chance = 1400}, -- small ruby
	{id = 2148, chance = 97000, maxCount = 75}, -- gold coin
	{id = 2178, chance = 520}, -- mind stone
	{id = 2194, chance = 690}, -- mysterious fetish
	{id = 2399, chance = 10000, maxCount = 3}, -- throwing star
	{id = 2417, chance = 4000}, -- battle hammer
	{id = 2459, chance = 3450}, -- iron helmet
	{id = 2513, chance = 5000}, -- battle shield
	{id = 2515, chance = 100}, -- guardian shield
	{id = 7618, chance = 10120, maxCount = 2}, -- health potion
	{id = 7618, chance = 10000, maxCount = 2}, -- health potion
	{id = 7620, chance = 5300}, -- mana potion
	{id = 10564, chance = 12600}, -- demonic skeletal hand
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -185, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -30, maxDamage = -50, range = 1, radius = 1, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)