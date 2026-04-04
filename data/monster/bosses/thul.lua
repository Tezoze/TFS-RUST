local mType = Game.createMonsterType("Thul")
local monster = {}

monster.description = "Thul"
monster.experience = 2700
monster.outfit = {
	lookType = 46,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6065
monster.health = 2950
monster.maxHealth = 2950
monster.race = "blood"
monster.speed = 520
monster.manaCost = 0
monster.maxSummons = 2

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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 40,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Gaaahhh!", yell = false},
	{text = "Boohaa!", yell = false},
}

monster.loot = {
	{id = 5895, chance = 100000}, -- fish fin
	{id = 2152, chance = 88000, maxCount = 10}, -- platinum coin
	{id = 7963, chance = 67000}, -- marlin
	{id = 7590, chance = 46000}, -- great mana potion
	{id = 2150, chance = 38000, maxCount = 4}, -- small amethyst
	{id = 7383, chance = 35000}, -- relic sword
	{id = 2497, chance = 16000}, -- crusader helmet
	{id = 2487, chance = 10000}, -- crown armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -285, target = false},
	{name = "combat", interval = 2000, chance = 7, minDamage = -108, maxDamage = -137, radius = 4, effect = CONST_ME_ICEAREA, target = false, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 25, minDamage = 0, maxDamage = -170, radius = 3, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "poisonfield", interval = 2000, chance = 19, radius = 3, shootEffect = CONST_ANI_POISON, target = true},
	{name = "speed", interval = 2000, chance = 18, range = 7, shootEffect = CONST_ANI_SNOWBALL, target = true, speed = -360, duration = 5000},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 2000, chance = 10, minDamage = 25, maxDamage = 75, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Massive Water Elemental", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)