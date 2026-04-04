local mType = Game.createMonsterType("The Obliverator")
local monster = {}

monster.description = "The Obliverator"
monster.experience = 6000
monster.outfit = {
	lookType = 35,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 9020
monster.maxHealth = 9020
monster.race = "fire"
monster.speed = 280
monster.manaCost = 0
monster.maxSummons = 3

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	targetDistance = 1,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 1000,
	chance = 10,
	{text = "NO ONE WILL BEAT ME!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -700, target = false},
	{name = "combat", interval = 1000, chance = 20, minDamage = -100, maxDamage = -250, range = 5, radius = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 3000, chance = 30, minDamage = -200, maxDamage = -500, effect = CONST_ME_ENERGY, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 40,
	{name = "combat", interval = 4000, chance = 5, minDamage = 50, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 50},
	{type = COMBAT_DEATHDAMAGE, percent = 1},
	{type = COMBAT_HOLYDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "fire elemental", chance = 50, interval = 2000, max = 3},
}

mType:register(monster)