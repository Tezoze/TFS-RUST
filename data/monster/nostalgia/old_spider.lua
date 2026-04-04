local mType = Game.createMonsterType("Old Spider")
local monster = {}

monster.description = "a spider"
monster.experience = 12
monster.outfit = {
	lookType = 922,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5961
monster.health = 20
monster.maxHealth = 20
monster.race = "venom"
monster.speed = 152
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
	runHealth = 6,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 65150, maxCount = 5}, -- gold coin
	{id = 8859, chance = 960}, -- spider fangs
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -9, target = false},
}

monster.defenses = {
	defense = 2,
	armor = 2,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -20},
}


mType:register(monster)