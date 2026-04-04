local mType = Game.createMonsterType("Gazer")
local monster = {}

monster.description = "a gazer"
monster.experience = 90
monster.outfit = {
	lookType = 109,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6036
monster.health = 120
monster.maxHealth = 120
monster.race = "venom"
monster.speed = 140
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Mommy!?", yell = false},
	{text = "Buuuuhaaaahhaaaaa!", yell = false},
	{text = "Me need mana!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 99350, maxCount = 16}, -- gold coin
	{id = 12468, chance = 3200}, -- small flask of eyedrops
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -15, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -25, maxDamage = -35, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -10, maxDamage = -35, range = 7, target = false, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 4,
	armor = 4,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 11},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)