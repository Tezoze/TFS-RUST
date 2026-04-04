local mType = Game.createMonsterType("Efreet")
local monster = {}

monster.description = "an efreet"
monster.experience = 410
monster.outfit = {
	lookType = 103,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6032
monster.health = 550
monster.maxHealth = 550
monster.race = "blood"
monster.speed = 234
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
	{text = "I grant you a deathwish!", yell = false},
	{text = "Good wishes are for fairytales", yell = false},
}

monster.loot = {
	{id = 1860, chance = 2200}, -- green tapestry
	{id = 2063, chance = 160}, -- small oil lamp
	{id = 2148, chance = 50000, maxCount = 75}, -- gold coin
	{id = 2148, chance = 60000, maxCount = 50}, -- gold coin
	{id = 2149, chance = 7000}, -- small emerald
	{id = 2155, chance = 200}, -- green gem
	{id = 2187, chance = 390}, -- wand of inferno
	{id = 2442, chance = 5000}, -- heavy machete
	{id = 2663, chance = 160}, -- mystic turban
	{id = 2673, chance = 9390, maxCount = 5}, -- pear
	{id = 5910, chance = 5000}, -- green piece of cloth
	{id = 7378, chance = 15570, maxCount = 3}, -- royal spear
	{id = 7589, chance = 3500}, -- strong mana potion
	{id = 7900, chance = 360}, -- magma monocle
	{id = 12426, chance = 8540}, -- jewelled belt
	{id = 12442, chance = 1130}, -- noble turban
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -40, maxDamage = -110, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -30, maxDamage = -90, radius = 3, effect = CONST_ME_ENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -65, maxDamage = -120, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -650, duration = 15000},
	{name = "drunk", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, duration = 6000},
	{name = "outfit", interval = 2000, chance = 1, range = 7, effect = CONST_ME_BLUESHIMMER, target = false},
	{name = "combat", interval = 2000, chance = 15, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 24,
	{name = "combat", interval = 2000, chance = 15, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 90},
	{type = COMBAT_ENERGYDAMAGE, percent = 60},
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -8},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "green djinn", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)