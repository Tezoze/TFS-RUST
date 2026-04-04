local mType = Game.createMonsterType("Shardhead")
local monster = {}

monster.description = "Shardhead"
monster.experience = 650
monster.outfit = {
	lookType = 261,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7282
monster.health = 800
monster.maxHealth = 800
monster.race = "undead"
monster.speed = 195
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 5
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 50,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.loot = {
	{id = 2148, chance = 99700, maxCount = 87}, -- gold coin
	{id = 7290, chance = 40000}, -- shard
	{id = 7588, chance = 100000}, -- strong health potion
	{id = 10578, chance = 40000}, -- frosty heart
	{id = 7441, chance = 80000}, -- ice cube
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 100, attack = 50, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -90, range = 7, shootEffect = CONST_ANI_ICE, effect = CONST_ME_ICEATTACK, target = true, type = COMBAT_ICEDAMAGE},
	{name = "speed", interval = 2000, chance = 12, effect = CONST_ME_ICEAREA, target = false, length = 8, spread = 0, speed = -360, duration = 5000},
}

monster.defenses = {
	defense = 26,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
	{type = COMBAT_ENERGYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "holy", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)