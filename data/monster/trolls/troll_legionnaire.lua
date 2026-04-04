local mType = Game.createMonsterType("Troll Legionnaire")
local monster = {}

monster.description = "a troll legionnaire"
monster.experience = 140
monster.outfit = {
	lookType = 53,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5998
monster.health = 210
monster.maxHealth = 210
monster.race = "blood"
monster.speed = 200
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 30,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 9,
	{text = "Attack!", yell = false},
	{text = "Graaaaar!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 92000, maxCount = 155}, -- gold coin
	{id = 2165, chance = 560}, -- stealth ring
	{id = 2399, chance = 28000, maxCount = 10}, -- throwing star
	{id = 10565, chance = 5120}, -- frosty ear of a troll
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
	{name = "combat", interval = 2000, chance = 18, minDamage = 0, maxDamage = -130, range = 6, shootEffect = CONST_ANI_THROWINGSTAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 9,
	armor = 12,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
	{name = "combat", interval = 2000, chance = 28, minDamage = 17, maxDamage = 25, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)