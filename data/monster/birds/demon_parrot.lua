local mType = Game.createMonsterType("Demon Parrot")
local monster = {}

monster.description = "a demon parrot"
monster.experience = 225
monster.outfit = {
	lookType = 217,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6056
monster.health = 360
monster.maxHealth = 360
monster.race = "blood"
monster.speed = 320
monster.manaCost = 250
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
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
	{text = "ISHH THAT THE BESHHT YOU HAVE TO OFFERRR, TIBIANSHH?", yell = false},
	{text = "YOU ARRRRRE DOOMED!", yell = false},
	{text = "I SHHMELL FEEAARRR!", yell = false},
	{text = "MY SHHEED IS FEARRR AND MY HARRRVEST ISHH YOURRR SHHOUL!", yell = false},
	{text = "Your shhoooul will be mineee!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 81630, maxCount = 99}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 1200, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 1000, chance = 30, effect = CONST_ME_REDNOTE, target = false, length = 5, spread = 0, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 30, minDamage = -25, maxDamage = -45, range = 5, shootEffect = CONST_ANI_SUDDENDEATH, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 1000, chance = 30, minDamage = -15, maxDamage = -45, range = 1, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 18,
	armor = 18,
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)