local mType = Game.createMonsterType("Lizard Noble")
local monster = {}

monster.description = "a lizard noble"
monster.experience = 2000
monster.outfit = {
	lookType = 115,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6041
monster.health = 7000
monster.maxHealth = 7000
monster.race = "blood"
monster.speed = 256
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Where are zhe guardz when you need zhem!", yell = false},
}

monster.loot = {
	{id = 2147, chance = 7100, maxCount = 5}, -- small ruby
	{id = 2148, chance = 91300, maxCount = 100}, -- gold coin
	{id = 2152, chance = 10000, maxCount = 20}, -- platinum coin
	{id = 5876, chance = 2220}, -- lizard leather
	{id = 5881, chance = 3650}, -- lizard scale
	{id = 7588, chance = 2550}, -- strong health potion
	{id = 7591, chance = 2900}, -- great health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = -120, maxDamage = -250, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -100, range = 7, effect = CONST_ME_BLUESHIMMER, target = false, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 15,
	armor = 27,
	{name = "combat", interval = 2000, chance = 50, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 90},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)