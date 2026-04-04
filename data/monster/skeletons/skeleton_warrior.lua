local mType = Game.createMonsterType("Skeleton Warrior")
local monster = {}

monster.description = "a skeleton warrior"
monster.experience = 45
monster.outfit = {
	lookType = 298,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5972
monster.health = 65
monster.maxHealth = 65
monster.race = "undead"
monster.speed = 154
monster.manaCost = 350
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 43500, maxCount = 10}, -- gold coin
	{id = 2230, chance = 50000}, -- bone
	{id = 2376, chance = 1500}, -- sword
	{id = 2398, chance = 2000}, -- mace
	{id = 2787, chance = 24000, maxCount = 3}, -- white mushroom
	{id = 2789, chance = 1700}, -- brown mushroom
	{id = 12437, chance = 10630}, -- pelvis bone
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -7, maxDamage = -13, range = 1, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
}


mType:register(monster)