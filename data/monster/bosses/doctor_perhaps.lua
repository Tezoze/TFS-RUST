local mType = Game.createMonsterType("Doctor Perhaps")
local monster = {}

monster.description = "doctor perhaps"
monster.experience = 325
monster.outfit = {
	lookType = 133,
	lookHead = 95,
	lookBody = 0,
	lookLegs = 94,
	lookFeet = 114,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 475
monster.maxHealth = 475
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I might use some parts of you in my next creation!", yell = false},
	{text = "You're only a testsubject to me!", yell = false},
	{text = "My creations will kill you!", yell = false},
	{text = "You can't beat what you can't comprehend!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 2000, maxCount = 95}, -- gold coin
	{id = 2152, chance = 30000, maxCount = 9}, -- platinum coin
	{id = 10316, chance = 1000}, -- mighty helm of green sparks
	{id = 10289, chance = 1000}, -- meat shield
	{id = 10290, chance = 1000}, -- glutton's mace
	{id = 10300, chance = 1000}, -- trousers of the ancients
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -43, interval = 2000, target = false},
	{name = "combat", type = COMBAT_DROWNDAMAGE, minDamage = -17, maxDamage = -55, interval = 2000, chance = 15, range = 5, radius = 3, target = true, shootEffect = CONST_ANI_SMALLEARTH, effect = CONST_ME_BLUEBUBBLE},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -20, maxDamage = -40, range = 7, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 15, minDamage = 10, maxDamage = 30, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}

monster.summons = {
	{name = "Zombie", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)