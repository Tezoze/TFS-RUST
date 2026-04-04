local mType = Game.createMonsterType("Bat")
local monster = {}

monster.description = ""
monster.experience = 10
monster.outfit = {
	lookType = 122,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6053
monster.health = 30
monster.maxHealth = 30
monster.race = "blood"
monster.speed = 230
monster.manaCost = 250
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
	runHealth = 3,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Flap!Flap!", yell = false},
}

monster.loot = {
	{id = 5894, chance = 15220}, -- bat wing
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -8, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -20},
}


mType:register(monster)