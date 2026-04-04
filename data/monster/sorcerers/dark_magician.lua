local mType = Game.createMonsterType("Dark Magician")
local monster = {}

monster.description = "a dark magician"
monster.experience = 185
monster.outfit = {
	lookType = 133,
	lookHead = 116,
	lookBody = 95,
	lookLegs = 50,
	lookFeet = 132,
	lookAddons = 2,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 325
monster.maxHealth = 325
monster.race = "blood"
monster.speed = 180
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Feel the power of my runes!", yell = false},
	{text = "Killing you gets expensive!", yell = false},
	{text = "My secrets are mine alone!", yell = false},
	{text = "Stand still!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 75100, maxCount = 55}, -- gold coin
	{id = 2260, chance = 10000}, -- blank rune
	{id = 7588, chance = 3000}, -- strong health potion
	{id = 7589, chance = 2860}, -- strong mana potion
	{id = 7618, chance = 12000}, -- health potion
	{id = 7620, chance = 11900}, -- mana potion
	{id = 13295, chance = 200}, -- reins
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -40, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -5, maxDamage = -40, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -20, maxDamage = -30, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 20, minDamage = 60, maxDamage = 80, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)