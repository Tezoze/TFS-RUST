local mType = Game.createMonsterType("Fluffy")
local monster = {}

monster.description = "Fluffy"
monster.experience = 3550
monster.outfit = {
	lookType = 240,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6332
monster.health = 4500
monster.maxHealth = 4500
monster.race = "blood"
monster.speed = 310
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
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

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Wooof!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 20}, -- gold coin
	{id = 5944, chance = 5555}, -- soul orb
	{id = 6570, chance = 5538, maxCount = 4},
	{id = 6571, chance = 1538},
	{id = 2671, chance = 50000, maxCount = 8}, -- ham
	{id = 2230, chance = 25000}, -- bone
	{id = 6500, chance = 7200}, -- demonic essence
	{id = 2430, chance = 2857}, -- knight axe
	{id = 2383, chance = 2500}, -- spike sword
	{id = 6558, chance = 8888}, -- concentrated demonic blood
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 98, attack = 120, target = false},
	{name = "combat", interval = 1500, chance = 30, minDamage = -100, maxDamage = -200, effect = CONST_ME_BLUEBUBBLE, target = false, length = 8, spread = 3, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 15, minDamage = -120, maxDamage = -300, effect = CONST_ME_POISON, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 25, minDamage = -105, maxDamage = -235, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 20, minDamage = -135, maxDamage = -255, range = 7, radius = 6, effect = CONST_ME_BLUEBUBBLE, target = false, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 25,
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)