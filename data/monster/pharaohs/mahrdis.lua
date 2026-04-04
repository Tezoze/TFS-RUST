local mType = Game.createMonsterType("Mahrdis")
local monster = {}

monster.description = "Mahrdis"
monster.experience = 3050
monster.outfit = {
	lookType = 90,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6025
monster.health = 3900
monster.maxHealth = 3900
monster.race = "undead"
monster.speed = 340
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ashes to ashes!", yell = false},
	{text = "Fire, Fire!", yell = false},
	{text = "The eternal flame demands its due!", yell = false},
	{text = "This is why I'm hot.", yell = false},
	{text = "May my flames engulf you!", yell = false},
	{text = "Burnnnnnnnnn!", yell = false},
}

monster.loot = {
	{id = 2141, chance = 500}, -- holy falcon
	{id = 2147, chance = 7000, maxCount = 3}, -- small ruby
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 70}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 64}, -- gold coin
	{id = 2156, chance = 1500}, -- red gem
	{id = 2168, chance = 1500}, -- life ring
	{id = 2353, chance = 100000}, -- burning heart
	{id = 2432, chance = 750}, -- fire axe
	{id = 2539, chance = 300}, -- phoenix shield
	{id = 7591, chance = 1500}, -- great health potion
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -400, interval = 2000, target = false},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = -60, maxDamage = -600, interval = 1600, chance = 7, range = 1, target = true, effect = CONST_ME_REDSHIMMER},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -60, maxDamage = -600, interval = 1000, chance = 7, range = 7, target = true, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA},
	{name = "speed", interval = 2000, chance = 13, range = 7, target = true, effect = CONST_ME_REDSHIMMER, speed = -850, duration = 50000},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -80, maxDamage = -800, interval = 2000, chance = 34, radius = 3, target = false, effect = CONST_ME_EXPLOSIONAREA},
	{name = "firefield", interval = 1000, chance = 12, radius = 4, target = false, effect = CONST_ME_YELLOWSPARK},
	{name = "condition", type = CONDITION_FIRE, interval = 2000, chance = 13, tick = 10000, minDamage = -50, maxDamage = -500, length = 8, spread = 3, effect = CONST_ME_EXPLOSIONAREA, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 20,
	{name = "combat", interval = 1000, chance = 20, minDamage = 20, maxDamage = 800, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 45},
	{type = COMBAT_HOLYDAMAGE, percent = -22},
	{type = COMBAT_ICEDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Fire Elemental", chance = 30, interval = 2000, max = 4},
}

mType:register(monster)