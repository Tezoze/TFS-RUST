local mType = Game.createMonsterType("Hive Pore")
local monster = {}

monster.description = "a hive pore"
monster.experience = 0
monster.outfit = {
	lookType = 0,
	lookTypeEx = 15467,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.health = 1
monster.maxHealth = 1
monster.race = "venom"
monster.speed = 0
monster.manaCost = 355
monster.maxSummons = 3

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = false,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.summons = {
	{name = "Lesser Swarmer", chance = 100, interval = 30000, max = 3},
}

monster.defenses = {
	defense = 0,
	armor = 0,
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
