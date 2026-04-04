local mType = Game.createMonsterType("Manta Ray")
local monster = {}

monster.description = "a manta ray"
monster.experience = 125
monster.outfit = {
	lookType = 449,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15276
monster.health = 680
monster.maxHealth = 680
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Flap flap flap!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 38}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -99, target = false, condition = {type = CONDITION_POISON, startDamage = 120, interval = 4000}},
	{name = "combat", interval = 2000, chance = 10, minDamage = -15, maxDamage = -75, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGYHIT, target = true, range = 7, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = 0, length = 4, spread = 0, effect = CONST_ME_ENERGYHIT, target = false, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -1},
	{type = COMBAT_PHYSICALDAMAGE, percent = -1},
	{type = COMBAT_ENERGYDAMAGE, percent = 1},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)
