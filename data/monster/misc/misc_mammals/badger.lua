local mType = Game.createMonsterType("Badger")
local monster = {}

monster.description = ""
monster.experience = 5
monster.outfit = {
	lookType = 105,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6034
monster.health = 23
monster.maxHealth = 23
monster.race = "blood"
monster.speed = 180
monster.manaCost = 200
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 11216, chance = 10230}, -- badger fur
	{id = 8845, chance = 40710}, -- beetroot
	{id = 11213, chance = 5130}, -- acorn
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -12, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}


mType:register(monster)