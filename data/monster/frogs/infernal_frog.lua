local mType = Game.createMonsterType("Infernal Frog")
local monster = {}

monster.description = "an infernal frog"
monster.experience = 190
monster.outfit = {
	lookType = 224,
	lookHead = 69,
	lookBody = 66,
	lookLegs = 69,
	lookFeet = 66,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6079
monster.health = 655
monster.maxHealth = 655
monster.race = "blood"
monster.speed = 200
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
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 40,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ribbit!", yell = false},
	{text = "Ribbit! Ribbit!", yell = false},
	{text = "No Kisses for you!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 77000, maxCount = 65}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = -16, maxDamage = -32, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 5,
	armor = 18,
	{name = "speed", interval = 2000, chance = 20, effect = CONST_ME_REDSHIMMER, speed = 400, duration = 8000},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)