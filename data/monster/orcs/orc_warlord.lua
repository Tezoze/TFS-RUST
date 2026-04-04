local mType = Game.createMonsterType("Orc Warlord")
local monster = {}

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
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ranat Ulderek!", yell = false},
	{text = "Orc buta bana!", yell = false},
	{text = "Ikem rambo zambo!", yell = false},
	{text = "Fetchi Maruk Buta", yell = false},
}

monster.loot = {
	{id = 2148, chance = 18000, maxCount = 45}, -- gold coin
	{id = 2165, chance = 90}, -- stealth ring
	{id = 2179, chance = 30}, -- gold ring
	{id = 2200, chance = 2190}, -- protection amulet
	{id = 2377, chance = 1680}, -- two handed sword
	{id = 2399, chance = 13920, maxCount = 18}, -- throwing star
	{id = 2419, chance = 3450}, -- scimitar
	{id = 2428, chance = 5400}, -- orcish axe
	{id = 2434, chance = 320}, -- dragon hammer
	{id = 2463, chance = 5210}, -- plate armor
	{id = 2465, chance = 740}, -- brass armor
	{id = 2490, chance = 1260}, -- dark helmet
	{id = 2497, chance = 280}, -- crusader helmet
	{id = 2647, chance = 4280}, -- plate legs
	{id = 2667, chance = 10800, maxCount = 2}, -- fish
	{id = 3965, chance = 5260}, -- hunting spear
	{id = 7395, chance = 50}, -- orc trophy
	{id = 7618, chance = 470}, -- health potion
	{id = 7891, chance = 280}, -- magma boots
	{id = 11113, chance = 9640}, -- orc tooth
	{id = 12409, chance = 24350}, -- broken helmet
	{id = 12435, chance = 20620}, -- orc leather
	{id = 12436, chance = 4610}, -- skull belt
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