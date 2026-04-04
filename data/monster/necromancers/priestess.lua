local mType = Game.createMonsterType("Priestess")
local monster = {}

monster.description = "a priestess"
monster.experience = 420
monster.outfit = {
	lookType = 58,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 390
monster.maxHealth = 390
monster.race = "blood"
monster.speed = 170
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
	illusionable = true,
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
	{text = "Your energy is mine.", yell = false},
	{text = "Now your life is come to the end, hahahaha!", yell = false},
	{text = "Throw the soul on the altar!", yell = false},
}

monster.loot = {
	{id = 1962, chance = 890},
	{id = 2070, chance = 1410}, -- wooden flute
	{id = 2114, chance = 90}, -- piggy bank
	{id = 2125, chance = 640}, -- crystal necklace
	{id = 2151, chance = 750}, -- talon
	{id = 2183, chance = 1100}, -- hailstorm rod
	{id = 2192, chance = 1250}, -- crystal ball
	{id = 2374, chance = 1400}, -- wooden flute
	{id = 2423, chance = 1500}, -- clerical mace
	{id = 2529, chance = 210}, -- black shield
	{id = 2674, chance = 7500, maxCount = 2}, -- red apple
	{id = 2760, chance = 11720}, -- goat grass
	{id = 2791, chance = 3240}, -- wood mushroom
	{id = 2802, chance = 13200}, -- sling herb
	{id = 2803, chance = 5900}, -- powder herb
	{id = 7620, chance = 850}, -- mana potion
	{id = 10556, chance = 1800}, -- cultish robe
	{id = 10562, chance = 5230}, -- black hood
	{id = 11220, chance = 9840}, -- dark rosary
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -75, interval = 2000, target = false},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = -55, maxDamage = -120, interval = 2000, chance = 20, range = 7, target = true, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA},
	{name = "combat", type = COMBAT_MANADRAIN, minDamage = -2, maxDamage = -170, interval = 2000, chance = 15, range = 7, target = true},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -200, maxDamage = -200, range = 7, shootEffect = CONST_ANI_POISON, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 30,
	{name = "combat", interval = 2000, chance = 15, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 40},
	{type = COMBAT_EARTHDAMAGE, percent = 70},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "ghoul", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)