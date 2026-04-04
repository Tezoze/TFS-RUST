local mType = Game.createMonsterType("Chizzoron The Distorter")
local monster = {}

monster.description = "Chizzoron The Distorter"
monster.experience = 4000
monster.outfit = {
	lookType = 340,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11316
monster.health = 16000
monster.maxHealth = 16000
monster.race = "blood"
monster.speed = 260
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 2000,
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
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 10,
	{text = "Humanzzz! Leave Zzaion at onzzzze!", yell = false},
	{text = "I pray to my mazzterzz, the mighty dragonzzz!", yell = false},
	{text = "You are not worzzy to touch zzizz zzacred ground!", yell = false},
}

monster.loot = {
	{id = 9971, chance = 71550, maxCount = 2}, -- gold ingot
	{id = 2148, chance = 69825, maxCount = 100}, -- gold coin
	{id = 2148, chance = 69825, maxCount = 10}, -- gold coin
	{id = 2149, chance = 5750}, -- small emerald
	{id = 5881, chance = 100000}, -- lizard scale
	{id = 2155, chance = 16300}, -- green gem
	{id = 2169, chance = 11025}, -- time ring
	{id = 7591, chance = 5750}, -- great health potion
	{id = 2492, chance = 5750}, -- dragon scale mail
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 60, attack = 130, target = false},
	{name = "combat", interval = 2000, chance = 20, maxDamage = -430, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, maxDamage = -874, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = false, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -300, maxDamage = -646, radius = 3, effect = CONST_ME_POFF, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -148, maxDamage = -250, range = 7, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 85,
	armor = 70,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Lizard Dragon Priest", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)