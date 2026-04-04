local mType = Game.createMonsterType("Ungreez")
local monster = {}

monster.description = "Ungreez"
monster.experience = 500
monster.outfit = {
	lookType = 35,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5995
monster.health = 8200
monster.maxHealth = 8200
monster.race = "blood"
monster.speed = 240
monster.manaCost = 10000
monster.maxSummons = 0

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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I teach you that even heroes can die!", yell = true},
	{text = "You will die begging like the others did!", yell = true},
}

monster.loot = {
	{id = 2148, chance = 21000, maxCount = 90}, -- gold coin
	{id = 2795, chance = 10000, maxCount = 6}, -- fire mushroom
	{id = 7590, chance = 20000}, -- great mana potion
	{id = 7591, chance = 20000}, -- great health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 70, attack = 120, target = false},
	{name = "combat", interval = 2000, chance = 13, minDamage = 0, maxDamage = -110, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, target = true, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 1000, chance = 14, minDamage = -150, maxDamage = -250, range = 7, radius = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 18, minDamage = -200, maxDamage = -400, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_PURPLEENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 12, minDamage = -300, maxDamage = -380, effect = CONST_ME_PURPLEENERGY, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 55,
	armor = 55,
	{name = "combat", interval = 2000, chance = 15, minDamage = 90, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
	{type = COMBAT_ICEDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)