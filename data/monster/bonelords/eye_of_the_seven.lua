local mType = Game.createMonsterType("Eye of the Seven")
local monster = {}

monster.description = "an eye of the seven"
monster.experience = 0
monster.outfit = {
	lookType = 109,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6036
monster.health = 1
monster.maxHealth = 1
monster.race = "venom"
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
	staticAttackChance = 90,
	runHealth = 100,
	healthHidden = false,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.attacks = {
	{name = "combat", interval = 2000, chance = 100, minDamage = 0, maxDamage = -500, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 1,
	armor = 1,
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
}


mType:register(monster)