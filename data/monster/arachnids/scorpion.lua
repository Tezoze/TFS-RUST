local mType = Game.createMonsterType("Scorpion")
local monster = {}

monster.description = "a scorpion"
monster.experience = 45
monster.outfit = {
	lookType = 43,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5988
monster.health = 45
monster.maxHealth = 45
monster.race = "venom"
monster.speed = 150
monster.manaCost = 310
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
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 5,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 10568, chance = 4930}, -- scorpion tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false, condition = {type = CONDITION_POISON, startDamage = 340, interval = 2000}},
}

monster.defenses = {
	defense = 5,
	armor = 14,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)