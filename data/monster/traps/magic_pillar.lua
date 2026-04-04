local mType = Game.createMonsterType("Magic Pillar")
local monster = {}

monster.description = "a magic pillar"
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

monster.corpse = 0
monster.health = 1
monster.maxHealth = 1
monster.race = "undead"
monster.speed = 0
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 2000,
	chance = 0
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
	runHealth = 100,
	healthHidden = true,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
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
	{type = "drunk", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
}

monster.summons = {
	{name = "Demon", chance = 7, interval = 2000, max = 3},
}

mType:register(monster)