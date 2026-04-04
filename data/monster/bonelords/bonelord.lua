local mType = Game.createMonsterType("Bonelord")
local monster = {}

monster.description = "a bonelord"
monster.experience = 170
monster.outfit = {
	lookType = 17,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5992
monster.health = 260
monster.maxHealth = 260
monster.race = "venom"
monster.speed = 150
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
	{text = "Eye for eye!", yell = false},
	{text = "Here's looking at you!", yell = false},
	{text = "Let me take a look at you.", yell = false},
	{text = "You've got the look!", yell = false},
	{text = "I've got to look!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 99400, maxCount = 48}, -- gold coin
	{id = 2175, chance = 4990}, -- spellbook
	{id = 2181, chance = 510}, -- terra rod
	{id = 2377, chance = 3950}, -- two handed sword
	{id = 2394, chance = 7020}, -- morning star
	{id = 2397, chance = 9060}, -- longsword
	{id = 2509, chance = 3980}, -- steel shield
	{id = 2518, chance = 110}, -- bonelord shield
	{id = 5898, chance = 1010}, -- bonelord eye
	{id = 7620, chance = 300}, -- mana potion
	{id = 12468, chance = 5060}, -- small flask of eyedrops
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -5, target = false},
	{name = "combat", interval = 2000, chance = 5, minDamage = -15, maxDamage = -45, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -25, maxDamage = -45, range = 7, shootEffect = CONST_ANI_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -30, maxDamage = -50, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -5, maxDamage = -45, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = -5, maxDamage = -50, range = 7, shootEffect = CONST_ANI_DEATH, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = 0, maxDamage = -45, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 5, minDamage = -5, maxDamage = -35, range = 7, target = false, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}

monster.summons = {
	{name = "Skeleton", chance = 10, interval = 4000, max = 6},
}

mType:register(monster)