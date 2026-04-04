local mType = Game.createMonsterType("Fire Elemental")
local monster = {}

monster.description = "a fire elemental"
monster.experience = 220
monster.outfit = {
	lookType = 49,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 1487
monster.health = 280
monster.maxHealth = 280
monster.race = "fire"
monster.speed = 190
monster.manaCost = 690
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -45, maxDamage = -160, range = 7, radius = 2, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 2000, chance = 25, range = 7, radius = 1, shootEffect = CONST_ANI_FIRE, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 18,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)