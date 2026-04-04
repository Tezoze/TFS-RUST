local mType = Game.createMonsterType("Spirit of Fire")
local monster = {}

monster.description = "a spirit of fire"
monster.experience = 950
monster.outfit = {
	lookType = 242,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 2210
monster.maxHealth = 2210
monster.race = "blood"
monster.speed = 220
monster.manaCost = 0
monster.maxSummons = 0

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
	interval = 5000,
	chance = 6,
	{text = "Feel the heat.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
	{name = "firefield", interval = 1000, chance = 15, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 1000, chance = 20, minDamage = 0, maxDamage = -300, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 0,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)