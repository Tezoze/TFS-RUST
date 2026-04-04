local mType = Game.createMonsterType("Evil Sheep")
local monster = {}

monster.description = "an evil sheep"
monster.experience = 240
monster.outfit = {
	lookType = 14,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5991
monster.health = 350
monster.maxHealth = 350
monster.race = "blood"
monster.speed = 156
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 20
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
	runHealth = 20,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grrrr", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 50}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false},
	{name = "combat", interval = 4000, chance = 30, minDamage = 0, maxDamage = -50, range = 7, shootEffect = CONST_ANI_SNOWBALL, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 18,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)