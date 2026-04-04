local mType = Game.createMonsterType("Hive Overseer")
local monster = {}

monster.description = "a hive overseer"
monster.experience = 5500
monster.outfit = {
	lookType = 458,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15354
monster.health = 7500
monster.maxHealth = 7500
monster.race = "venom"
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Zopp!", yell = false},
	{text = "Kropp!", yell = false},
}

monster.summons = {
	{name = "Spidris Elite", chance = 40, interval = 2000, max = 2},
}

monster.loot = {
	{id = 2147, chance = 16000, maxCount = 2}, -- small ruby
	{id = 2148, chance = 40000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 58000, maxCount = 98}, -- gold coin
	{id = 2152, chance = 84000, maxCount = 6}, -- platinum coin
	{id = 2645, chance = 550}, -- steel boots
	{id = 7590, chance = 18000}, -- great mana potion
	{id = 7632, chance = 6000}, -- giant shimmering pearl
	{id = 8473, chance = 12000}, -- ultimate health potion
	{id = 9971, chance = 29000}, -- gold ingot
	{id = 15480, chance = 28000}, -- kollos shell
	{id = 15486, chance = 16000}, -- compound eye
	{id = 15489, chance = 830}, -- calopteryx cape
	{id = 15491, chance = 920}, -- carapace shield
	{id = 15492, chance = 1650}, -- hive scythe
	{id = 15572, chance = 13000, maxCount = 2}, -- gooey mass
	{id = 15643, chance = 830}, -- hive bow
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -450, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -60, maxDamage = -80, radius = 4, effect = CONST_ME_POISONAREA, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -600, maxDamage = -1000, radius = 4, effect = CONST_ME_GREENBUBBLE, target = false, type = COMBAT_EARTHDAMAGE, condition = {type = CONDITION_POISON, startDamage = 600, interval = 4000}},
}

monster.defenses = {
	defense = 45,
	armor = 45,
	{name = "combat", interval = 2000, chance = 50, minDamage = 50, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 2000, chance = 15, minDamage = 500, maxDamage = 700, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 70},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 60},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
