local mType = Game.createMonsterType("Elder Bonelord")
local monster = {}

monster.description = "an elder bonelord"
monster.experience = 280
monster.outfit = {
	lookType = 108,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6037
monster.health = 500
monster.maxHealth = 500
monster.race = "blood"
monster.speed = 170
monster.manaCost = 0
monster.maxSummons = 6

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
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Let me take a look at you!", yell = false},
	{text = "Inferior creatures, bow before my power!", yell = false},
	{text = "659978 54764!", yell = false},
	{text = "653768764!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 99290, maxCount = 86}, -- gold coin
	{id = 11193, chance = 20040}, -- elder bonelord tentacle
	{id = 12468, chance = 9720}, -- small flask of eyedrops
	{id = 7364, chance = 8780, maxCount = 5}, -- sniper arrow
	{id = 2377, chance = 2980}, -- two handed sword
	{id = 2509, chance = 2040}, -- steel shield
	{id = 2175, chance = 1030}, -- spellbook
	{id = 7589, chance = 830}, -- strong mana potion
	{id = 11197, chance = 460}, -- giant eye
	{id = 2518, chance = 90}, -- bonelord shield
	{id = 3972, chance = 90}, -- bonelord helmet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -55, target = false},
	{name = "combat", interval = 2000, chance = 5, minDamage = -45, maxDamage = -60, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -40, maxDamage = -80, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -45, maxDamage = -90, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -20, maxDamage = -40, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -45, maxDamage = -85, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 5, minDamage = 0, maxDamage = -40, range = 7, target = false, type = COMBAT_MANADRAIN},
	{name = "speed", interval = 2000, chance = 10, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -600, duration = 20000},
}

monster.defenses = {
	defense = 13,
	armor = 13,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Gazer", chance = 10, interval = 2000, max = 6},
	{name = "Crypt Shambler", chance = 15, interval = 2000, max = 6},
}

mType:register(monster)