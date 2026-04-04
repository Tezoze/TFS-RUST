local mType = Game.createMonsterType("Fernfang")
local monster = {}

monster.description = "Fernfang"
monster.experience = 600
monster.outfit = {
	lookType = 206,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 400
monster.maxHealth = 400
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 3

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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "You desacrated this place!", yell = false},
	{text = "I will cleanse this isle!", yell = false},
	{text = "Grrrrrrr", yell = false},
	{text = "Yoooohuuuu!", yell = true},
}

monster.loot = {
	{id = 10563, chance = 100000}, -- book of prayers
	{id = 2148, chance = 100000, maxCount = 95}, -- gold coin
	{id = 2152, chance = 93000, maxCount = 3}, -- platinum coin
	{id = 2800, chance = 86000}, -- star herb
	{id = 12448, chance = 53000}, -- rope belt
	{id = 2166, chance = 40000}, -- power ring
	{id = 12449, chance = 40000}, -- safety pin
	{id = 2154, chance = 33000}, -- yellow gem
	{id = 2015, chance = 20000}, -- brown flask
	{id = 7589, chance = 20000}, -- strong mana potion
	{id = 2044, chance = 13000}, -- lamp
	{id = 2401, chance = 13000}, -- staff
	{id = 5786, chance = 13000}, -- wooden whistle
	{id = 2260, chance = 7000}, -- blank rune
	{id = 2689, chance = 7000}, -- bread
	{id = 2652, chance = 7000}, -- green tunic
	{id = 2177, chance = 7000}, -- life crystal
	{id = 2802, chance = 7000}, -- sling herb
	{id = 2129, chance = 7000}, -- wolf tooth chain
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
	{name = "combat", interval = 1000, chance = 13, minDamage = -65, maxDamage = -180, range = 7, shootEffect = CONST_ANI_SMALLHOLY, effect = CONST_ME_HOLYDAMAGE, target = true, type = COMBAT_HOLYDAMAGE},
	{name = "combat", interval = 1000, chance = 25, minDamage = -20, maxDamage = -45, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 10,
	armor = 15,
	{name = "combat", interval = 2000, chance = 15, minDamage = 10, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 7, effect = CONST_ME_REDSHIMMER, speed = 280, duration = 10000},
	{name = "outfit", interval = 1000, chance = 5, effect = CONST_ME_BLUESHIMMER, monster = "War Wolf", duration = 14000},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 70},
	{type = COMBAT_EARTHDAMAGE, percent = 40},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "War Wolf", chance = 13, interval = 1000, max = 3},
}

mType:register(monster)