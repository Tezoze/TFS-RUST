local mType = Game.createMonsterType("Dark Apprentice")
local monster = {}

monster.description = "a dark apprentice"
monster.experience = 100
monster.outfit = {
	lookType = 133,
	lookHead = 78,
	lookBody = 57,
	lookLegs = 95,
	lookFeet = 115,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 225
monster.maxHealth = 225
monster.race = "blood"
monster.speed = 172
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
	{text = "Outch!", yell = false},
	{text = "Oops, I did it again.", yell = false},
	{text = "From the spirits that I called Sir, deliver me!", yell = false},
	{text = "I must dispose of my masters enemies!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 7500, maxCount = 45}, -- gold coin
	{id = 2188, chance = 110}, -- wand of decay
	{id = 2191, chance = 1980}, -- wand of dragonbreath
	{id = 2260, chance = 8125, maxCount = 3}, -- blank rune
	{id = 7618, chance = 2900}, -- health potion
	{id = 7620, chance = 2980}, -- mana potion
	{id = 13295, chance = 110}, -- reins
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -2, maxDamage = -26, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -10, maxDamage = -20, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -24, range = 7, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "outfit", interval = 2000, chance = 1, range = 3, shootEffect = CONST_ANI_EXPLOSION, target = true},
	{name = "outfit", interval = 2000, chance = 1, radius = 4, effect = CONST_ME_BLUESHIMMER, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 16,
	{name = "combat", interval = 2000, chance = 15, minDamage = 30, maxDamage = 40, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "outfit", interval = 2000, chance = 5, monster = "green frog", duration = 3000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)