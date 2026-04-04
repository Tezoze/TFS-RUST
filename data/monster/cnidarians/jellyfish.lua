local mType = Game.createMonsterType("Jellyfish")
local monster = {}

monster.description = "a jellyfish"
monster.experience = 0
monster.outfit = {
	lookType = 452,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15284
monster.health = 55
monster.maxHealth = 55
monster.race = "undead"
monster.speed = 170
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Luuurrrp", yell = false},
}

monster.loot = {
	{id = 2669, chance = 8333}, -- shrimp
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -10, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 10,
	{name = "invisible", interval = 2000, chance = 15, duration = 3000},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)
