local mType = Game.createMonsterType("Lavahole")
local monster = {}

monster.description = "a lavahole"
monster.experience = 0
monster.outfit = {
	lookType = 0,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.health = 1
monster.maxHealth = 1
monster.race = "undead"
monster.speed = 0
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = false,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = false,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 100,
	runHealth = 0,
	healthHidden = true,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.attacks = {
	{name = "combat", interval = 2000, chance = 50, minDamage = 0, maxDamage = -100, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.immunities = {
	{type = "physical", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "holy", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)