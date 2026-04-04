local mType = Game.createMonsterType("Phantasm")
local monster = {}

monster.description = "a phantasm"
monster.experience = 4400
monster.outfit = {
	lookType = 241,
	lookHead = 20,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6344
monster.health = 3950
monster.maxHealth = 3950
monster.race = "undead"
monster.speed = 340
monster.manaCost = 0
monster.maxSummons = 4

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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 350,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Oh my, you forgot to put your pants on!", yell = false},
	{text = "Weeheeheeheehee!", yell = false},
	{text = "Its nothing but a dream.", yell = false},
	{text = "Dream a little dream with me!", yell = false},
	{text = "Give in.", yell = false},
}

monster.loot = {
	{id = 2147, chance = 12160, maxCount = 3}, -- small ruby
	{id = 2148, chance = 30000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 30000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 28000, maxCount = 33}, -- gold coin
	{id = 2149, chance = 10190, maxCount = 3}, -- small emerald
	{id = 2150, chance = 14570, maxCount = 3}, -- small amethyst
	{id = 2152, chance = 87730, maxCount = 4}, -- platinum coin
	{id = 2165, chance = 550}, -- stealth ring
	{id = 2260, chance = 22500, maxCount = 2}, -- blank rune
	{id = 2487, chance = 660}, -- crown armor
	{id = 2804, chance = 26930, maxCount = 2}, -- shadow herb
	{id = 6300, chance = 330}, -- death ring
	{id = 6500, chance = 16320}, -- demonic essence
	{id = 7414, chance = 110}, -- abyss hammer
	{id = 7451, chance = 550}, -- shadow sceptre
	{id = 7590, chance = 32750, maxCount = 2}, -- great mana potion
	{id = 8473, chance = 14680}, -- ultimate health potion
	{id = 9970, chance = 12810, maxCount = 3}, -- small topaz
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -475, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -250, maxDamage = -610, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -5, maxDamage = -80, radius = 3, effect = CONST_ME_YELLOWBUBBLE, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 15, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, radius = 5, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 0,
	{name = "combat", interval = 2000, chance = 30, minDamage = 228, maxDamage = 449, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 10, effect = CONST_ME_REDSHIMMER, speed = 500, duration = 6000},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "physical", combat = true, condition = true},
}

monster.summons = {
	{name = "Phantasm Summon", chance = 20, interval = 2000, max = 4},
}

mType:register(monster)