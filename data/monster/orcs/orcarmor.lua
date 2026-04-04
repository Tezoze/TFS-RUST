local mType = Game.createMonsterType("Orcarmor")
local monster = {}

monster.name = "Orc Warlord"
monster.description = "an orc warlord"
monster.experience = 670
monster.outfit = {
	lookType = 2,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6008
monster.health = 950
monster.maxHealth = 950
monster.race = "blood"
monster.speed = 234
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
	{text = "Ranat Ulderek!", yell = false},
	{text = "Orc buta bana!", yell = false},
	{text = "Ikem rambo zambo!", yell = false},
	{text = "Fetchi Maruk Buta", yell = false},
}

monster.loot = {
	{id = 12409, chance = 25000}, -- broken helmet
	{id = 12435, chance = 20000}, -- orc leather
	{id = 2148, chance = 18000, maxCount = 45}, -- gold coin
	{id = 2399, chance = 14000, maxCount = 18}, -- throwing star
	{id = 2667, chance = 10800, maxCount = 2}, -- fish
	{id = 11113, chance = 9500}, -- orc tooth
	{id = 2428, chance = 5200}, -- orcish axe
	{id = 3965, chance = 5200}, -- hunting spear
	{id = 2463, chance = 5110}, -- plate armor
	{id = 12436, chance = 4610}, -- skull belt
	{id = 2647, chance = 4180}, -- plate legs
	{id = 2419, chance = 3550}, -- scimitar
	{id = 2200, chance = 2190}, -- protection amulet
	{id = 2377, chance = 1800}, -- two handed sword
	{id = 2490, chance = 1400}, -- dark helmet
	{id = 2465, chance = 670}, -- brass armor
	{id = 7618, chance = 420}, -- health potion
	{id = 2497, chance = 340}, -- crusader helmet
	{id = 2434, chance = 290}, -- dragon hammer
	{id = 7891, chance = 250}, -- magma boots
	{id = 7395, chance = 80}, -- orc trophy
	{id = 2165, chance = 80}, -- stealth ring
	{id = 2500, chance = 5000}, -- amazon armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -200, range = 7, shootEffect = CONST_ANI_THROWINGSTAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 35,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 80},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)