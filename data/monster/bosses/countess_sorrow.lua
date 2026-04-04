local mType = Game.createMonsterType("Countess Sorrow")
local monster = {}

monster.description = "Countess Sorrow"
monster.experience = 13000
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
monster.health = 6500
monster.maxHealth = 6500
monster.race = "undead"
monster.speed = 400
monster.manaCost = 0
monster.maxSummons = 3

monster.changeTarget = {
	interval = 60000,
	chance = 0
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
	runHealth = 540,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "I'm so sorry ... for youuu!", yell = false},
	{text = "You won't rest in peace! Never ever!", yell = false},
	{text = "Sleep ... for eternity!", yell = false},
	{text = "Dreams can come true. As my dream of killing you.", yell = false},
}

monster.loot = {
	{id = 6536, chance = 100000}, -- countess sorrow's frozen tear
	{id = 6500, chance = 20590}, -- demonic essence
	{id = 2148, chance = 82350, maxCount = 169}, -- gold coin
	{id = 2152, chance = 55880, maxCount = 4}, -- platinum coin
	{id = 5944, chance = 85290}, -- soul orb
	{id = 2656, chance = 32350}, -- blue robe
	{id = 2424, chance = 4210}, -- silver mace
	{id = 2647, chance = 8820}, -- plate legs
	{id = 2200, chance = 23530}, -- protection amulet
	{id = 2165, chance = 5880}, -- stealth ring
	{id = 2238, chance = 47060}, -- worn leather boots
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 156, attack = 100, target = false, condition = {type = CONDITION_POISON, startDamage = 920, interval = 2000}},
	{name = "combat", interval = 2000, chance = 10, minDamage = -420, maxDamage = -980, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENSPARK, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 12, minDamage = -45, maxDamage = -90, radius = 3, effect = CONST_ME_YELLOWBUBBLE, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 20, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, range = 7, radius = 6, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 25,
	{name = "combat", interval = 2000, chance = 26, minDamage = 415, maxDamage = 625, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_POFF},
	{name = "speed", interval = 2000, chance = 11, effect = CONST_ME_REDSHIMMER, speed = 736, duration = 6000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = 50},
}

monster.immunities = {
	{type = "physical", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "poison", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Phantasm", chance = 7, interval = 2000, max = 3},
	{name = "Phantasm summon", chance = 7, interval = 2000, max = 3},
}

mType:register(monster)