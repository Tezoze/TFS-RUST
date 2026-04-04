local mType = Game.createMonsterType("Werewolf")
local monster = {}

monster.description = "a werewolf"
monster.experience = 1900
monster.outfit = {
	lookType = 308,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 1955
monster.maxHealth = 1955
monster.race = "blood"
monster.speed = 280
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 80,
	runHealth = 300,
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "GRRR", yell = true},
	{text = "GRROARR", yell = true},
}

monster.loot = {
	{id = 2148, chance = 98000, maxCount = 230}, -- gold coin
	{id = 2169, chance = 800}, -- time ring
	{id = 2171, chance = 870}, -- platinum amulet
	{id = 2197, chance = 1000}, -- stone skin amulet
	{id = 2381, chance = 3000}, -- halberd
	{id = 2438, chance = 560}, -- epee
	{id = 2510, chance = 10340}, -- plate shield
	{id = 2789, chance = 6940}, -- brown mushroom
	{id = 2805, chance = 1900}, -- troll green
	{id = 5897, chance = 5200}, -- wolf paw
	{id = 7383, chance = 480}, -- relic sword
	{id = 7419, chance = 160}, -- dreaded cleaver
	{id = 7428, chance = 400}, -- bonebreaker
	{id = 7439, chance = 1200}, -- berserk potion
	{id = 7588, chance = 5000}, -- strong health potion
	{id = 8473, chance = 2400}, -- ultimate health potion
	{id = 9809, chance = 210},
	{id = 8859, chance = 15000}, -- spider fangs
	{id = 11234, chance = 10650}, -- werewolf fur
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -350, target = false},
	{name = "outfit", interval = 2000, chance = 1, radius = 1, target = true},
	{name = "combat", interval = 2000, chance = 10, minDamage = -80, maxDamage = -200, effect = CONST_ME_REDNOTE, target = false, length = 4, spread = 2, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 40, radius = 3, effect = CONST_ME_WHITENOTE, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, radius = 1, effect = CONST_ME_GREENNOTE, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, range = 1, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 36,
	{name = "combat", interval = 2000, chance = 15, minDamage = 120, maxDamage = 225, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_PURPLENOTE, speed = 400, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 5},
	{type = COMBAT_EARTHDAMAGE, percent = 65},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "war wolf", chance = 40, interval = 2000, max = 2},
}

mType:register(monster)