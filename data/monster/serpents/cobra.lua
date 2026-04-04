local mType = Game.createMonsterType("Cobra")
local monster = {}

monster.description = ""
monster.experience = 30
monster.outfit = {
	lookType = 81,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 3007
monster.health = 65
monster.maxHealth = 65
monster.race = "blood"
monster.speed = 120
monster.manaCost = 275
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
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Zzzzzz", yell = false},
	{text = "Fsssss", yell = false},
}

monster.loot = {
	{id = 10551, chance = 5000}, -- cobra tongue
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = 0, interval = 2000, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -20, maxDamage = -40, range = 7, target = true},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "drunk", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)