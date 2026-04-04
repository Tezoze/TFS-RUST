local mType = Game.createMonsterType("Morik The Gladiator")
local monster = {}

monster.description = "Morik The Gladiator"
monster.experience = 160
monster.outfit = {
	lookType = 131,
	lookHead = 57,
	lookBody = 57,
	lookLegs = 95,
	lookFeet = 95,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 1235
monster.maxHealth = 1235
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 2000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 10,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "To be the one you'll have to beat the one!", yell = false},
	{text = "Where did I put my ultimate health potion again?", yell = false},
	{text = "I am the best!", yell = false},
	{text = "I'll take your ears as a trophy!", yell = false},
}

monster.loot = {
	{id = 9735, chance = 100000}, -- morik's helmet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false},
	{name = "combat", interval = 2000, chance = 15, maxDamage = -110, radius = 3, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 3000, chance = 34, range = 7, shootEffect = CONST_ANI_WHIRLWINDSWORD, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 22,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Gladiator", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)