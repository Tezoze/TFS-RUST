local mType = Game.createMonsterType("Old Bug")
local monster = {}

monster.description = ""
monster.experience = 18
monster.outfit = {
	lookType = 920,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5990
monster.health = 29
monster.maxHealth = 29
monster.race = "venom"
monster.speed = 160
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 51170, maxCount = 6}, -- gold coin
	{id = 2679, chance = 2590, maxCount = 2}, -- cherry
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -23, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}


mType:register(monster)