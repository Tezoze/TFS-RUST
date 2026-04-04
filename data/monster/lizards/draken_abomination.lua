local mType = Game.createMonsterType("Draken Abomination")
local monster = {}

monster.description = "a draken abomination"
monster.experience = 3800
monster.outfit = {
	lookType = 357,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 12623
monster.health = 6250
monster.maxHealth = 6250
monster.race = "venom"
monster.speed = 270
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Uhhhg!", yell = false},
	{text = "Hmmnn!", yell = false},
	{text = "Aaag!", yell = false},
	{text = "Gll", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 47000, maxCount = 98}, -- gold coin
	{id = 2152, chance = 50590, maxCount = 8}, -- platinum coin
	{id = 2666, chance = 50450, maxCount = 4}, -- meat
	{id = 7590, chance = 9950, maxCount = 3}, -- great mana potion
	{id = 7903, chance = 8730}, -- terra hood
	{id = 8472, chance = 4905, maxCount = 3}, -- great spirit potion
	{id = 8473, chance = 9400, maxCount = 3}, -- ultimate health potion
	{id = 8922, chance = 1020}, -- wand of voodoo
	{id = 9970, chance = 2900, maxCount = 4}, -- small topaz
	{id = 11301, chance = 470}, -- Zaoan armor
	{id = 11302, chance = 560}, -- Zaoan helmet
	{id = 11304, chance = 780}, -- Zaoan legs
	{id = 12627, chance = 12110}, -- eye of corruption
	{id = 12628, chance = 6240}, -- tail of corruption
	{id = 12629, chance = 10940}, -- scale of corruption
	{id = 12644, chance = 10}, -- shield of corruption
	{id = 12646, chance = 540}, -- draken boots
	{id = 12647, chance = 10}, -- snake god's wristguard
	{id = 11134, chance = 1000}, -- Tome of Knowledge
	{id = 13538, chance = 310}, -- bamboo leaves
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -420, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -310, maxDamage = -630, effect = CONST_ME_EXPLOSION, target = false, length = 4, spread = 3, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -170, maxDamage = -370, effect = CONST_ME_MORTAREA, target = false, length = 4, spread = 0, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, range = 7, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, range = 7, radius = 3, effect = CONST_ME_GREENSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 45,
	{name = "combat", interval = 2000, chance = 15, minDamage = 650, maxDamage = 700, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = 5},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Death Blob", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)