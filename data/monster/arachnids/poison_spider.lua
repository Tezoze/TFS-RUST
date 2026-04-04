local mType = Game.createMonsterType("Poison Spider")
local monster = {}

monster.description = "a poison spider"
monster.experience = 22
monster.outfit = {
	lookType = 36,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5974
monster.health = 26
monster.maxHealth = 26
monster.race = "venom"
monster.speed = 160
monster.manaCost = 270
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
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 6,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 75000, maxCount = 4}, -- gold coin
	{id = 12441, chance = 1140}, -- poison spider shell
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false, condition = {type = CONDITION_POISON, startDamage = 30, interval = 2000}},
}

monster.defenses = {
	defense = 5,
	armor = 2,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)