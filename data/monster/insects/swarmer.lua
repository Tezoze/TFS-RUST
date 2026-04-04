local mType = Game.createMonsterType("Swarmer")
local monster = {}

monster.description = "a swarmer"
monster.experience = 350
monster.outfit = {
	lookType = 460,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15388
monster.health = 460
monster.maxHealth = 460
monster.race = "venom"
monster.speed = 190
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 50,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Flzlzlzlzlzlzlz!", yell = false},
	{text = "Rzlrzlrzlrzlrzlrzl!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 75}, -- gold coin
	{id = 2149, chance = 920}, -- small emerald
	{id = 2438, chance = 450}, -- epee
	{id = 15479, chance = 15300}, -- swarmer antenna
	{id = 15486, chance = 12500}, -- compound eye
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -102, target = false, condition = {type = CONDITION_POISON, startDamage = 80, interval = 4000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -50, maxDamage = -110, effect = CONST_ME_MAGIC_RED, target = true, range = 7, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 10,
	armor = 10,
	{name = "speed", interval = 2000, chance = 15, speed = 220, effect = CONST_ME_MAGIC_RED, target = false, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 75},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
