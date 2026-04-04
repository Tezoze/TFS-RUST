local mType = Game.createMonsterType("Deathslicer")
local monster = {}

monster.description = "a deathslicer"
monster.experience = 0
monster.outfit = {
	lookType = 102,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 2253
monster.health = 1
monster.maxHealth = 1
monster.race = "undead"
monster.speed = 200
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -500, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = -200, maxDamage = -400, radius = 2, effect = CONST_ME_YELLOWSPARK, target = false, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
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