local mType = Game.createMonsterType("Hellspawn")
local monster = {}

monster.description = "a hellspawn"
monster.experience = 2550
monster.outfit = {
	lookType = 322,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9923
monster.health = 3500
monster.maxHealth = 3500
monster.race = "fire"
monster.speed = 344
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 15
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
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Your fragile bones are like toothpicks to me.", yell = false},
	{text = "You little weasel will not live to see another day.", yell = false},
	{text = "I'm just a messenger of what's yet to come.", yell = false},
	{text = "HRAAAAAAAAAAAAAAAARRRR!", yell = true},
	{text = "I'm taking you down with me!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 60000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 60000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 60000, maxCount = 36}, -- gold coin
	{id = 2394, chance = 10000}, -- morning star
	{id = 2475, chance = 1886}, -- warrior helmet
	{id = 2477, chance = 3030}, -- knight legs
	{id = 2788, chance = 7692, maxCount = 2}, -- red mushroom
	{id = 6500, chance = 9090}, -- demonic essence
	{id = 7368, chance = 9090, maxCount = 2}, -- assassin star
	{id = 7421, chance = 103}, -- onyx flail
	{id = 7439, chance = 934}, -- berserk potion
	{id = 7452, chance = 970}, -- spiked squelcher
	{id = 7591, chance = 40333}, -- great health potion
	{id = 8473, chance = 9090}, -- ultimate health potion
	{id = 9809, chance = 3125},
	{id = 9810, chance = 3125},
	{id = 9948, chance = 140}, -- dracoyle statue
	{id = 9969, chance = 151}, -- black skull
	{id = 9970, chance = 5882, maxCount = 3}, -- small topaz
	{id = 11221, chance = 20000}, -- hellspawn tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -352, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -150, maxDamage = -175, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREATTACK, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 44,
	{name = "combat", interval = 2000, chance = 10, minDamage = 120, maxDamage = 230, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 270, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 40},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 80},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)