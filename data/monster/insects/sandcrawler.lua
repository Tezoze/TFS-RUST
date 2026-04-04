local mType = Game.createMonsterType("Sandcrawler")
local monster = {}

monster.description = "a sandcrawler"
monster.experience = 20
monster.outfit = {
	lookType = 350,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11357
monster.health = 30
monster.maxHealth = 30
monster.race = "venom"
monster.speed = 160
monster.manaCost = 250
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Chrk chrk!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 33333, maxCount = 6}, -- gold coin
	{id = 11373, chance = 2173}, -- sandcrawler shell
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -3, target = false},
}

monster.defenses = {
	defense = 10,
	armor = 2,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -5},
}


mType:register(monster)