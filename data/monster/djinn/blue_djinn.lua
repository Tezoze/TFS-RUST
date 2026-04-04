local mType = Game.createMonsterType("Blue Djinn")
local monster = {}

monster.description = "a blue djinn"
monster.experience = 215
monster.outfit = {
	lookType = 80,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6020
monster.health = 330
monster.maxHealth = 330
monster.race = "blood"
monster.speed = 220
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
	{text = "Simsalabim", yell = false},
	{text = "Feel the power of my magic, tiny mortal!", yell = false},
	{text = "Be careful what you wish for.", yell = false},
	{text = "Wishes can come true", yell = false},
}

monster.loot = {
	{id = 1963, chance = 2350},
	{id = 2063, chance = 690}, -- small oil lamp
	{id = 2146, chance = 2560, maxCount = 4}, -- small sapphire
	{id = 2148, chance = 60000, maxCount = 70}, -- gold coin
	{id = 2148, chance = 70000, maxCount = 45}, -- gold coin
	{id = 2663, chance = 70}, -- mystic turban
	{id = 2684, chance = 23480}, -- carrot
	{id = 2745, chance = 440}, -- blue rose
	{id = 5912, chance = 5920}, -- blue piece of cloth
	{id = 7378, chance = 4500, maxCount = 2}, -- royal spear
	{id = 7620, chance = 860}, -- mana potion
	{id = 12412, chance = 1890}, -- dirty turban
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -45, maxDamage = -80, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -60, maxDamage = -105, range = 7, radius = 1, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "drunk", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, duration = 5000},
	{name = "outfit", interval = 2000, chance = 1, range = 7, effect = CONST_ME_BLUESHIMMER, target = true, duration = 4000, monster = "rat"},
}

monster.defenses = {
	defense = 15,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 80},
	{type = COMBAT_ENERGYDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -12},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)