local mType = Game.createMonsterType("The Count")
local monster = {}

monster.description = "the Count"
monster.experience = 450
monster.outfit = {
	lookType = 287,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8937
monster.health = 1250
monster.maxHealth = 1250
monster.race = "undead"
monster.speed = 370
monster.manaCost = 0
monster.maxSummons = 1

monster.changeTarget = {
	interval = 5000,
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 40000, maxCount = 98}, -- gold coin
	{id = 8752, chance = 100000}, -- the ring of the count
	{id = 2391, chance = 2300}, -- war hammer
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = -80, maxDamage = -135, skill = 120, target = false},
	{name = "combat", interval = 1000, chance = 9, minDamage = 0, maxDamage = -300, radius = 4, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 1000, chance = 25, minDamage = 100, maxDamage = 195, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 3000, chance = 30, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Banshee", chance = 50, interval = 4000, max = 1},
}

mType:register(monster)