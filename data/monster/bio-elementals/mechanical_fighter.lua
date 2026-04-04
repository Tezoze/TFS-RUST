local mType = Game.createMonsterType("Mechanical Fighter")
local monster = {}

monster.description = "a mechanical fighter"
monster.experience = 255
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
monster.health = 420
monster.maxHealth = 420
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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true,
}

monster.loot = {
	{id = 5901, chance = 87460}, -- wood
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -500, target = false},
}

monster.defenses = {
	defense = 199,
	armor = 199,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 50},
}

monster.immunities = {
	{type = "holy", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)