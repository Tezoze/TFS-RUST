local mType = Game.createMonsterType("Evil Sheep Lord")
local monster = {}

monster.description = "an evil sheep lord"
monster.experience = 340
monster.outfit = {
	lookType = 13,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5994
monster.health = 400
monster.maxHealth = 400
monster.race = "blood"
monster.speed = 178
monster.manaCost = 0
monster.maxSummons = 3

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
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You can COUNT on us!", yell = false},
	{text = "Maeh!", yell = false},
	{text = "I feel you're getting sleepy! Maeh!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 75000, maxCount = 60}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -118, target = false},
	{name = "outfit", interval = 3000, chance = 20, range = 7, effect = CONST_ME_BLUESHIMMER, target = true},
}

monster.defenses = {
	defense = 35,
	armor = 24,
	{name = "combat", interval = 2000, chance = 20, minDamage = 50, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "outfit", interval = 1500, chance = 50, effect = CONST_ME_BLUESHIMMER, monster = "Werewolf", duration = 3000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Evil Sheep", chance = 30, interval = 2000, max = 3},
}

mType:register(monster)