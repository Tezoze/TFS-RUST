local mType = Game.createMonsterType("Worker Golem")
local monster = {}

monster.description = "a worker golem"
monster.experience = 1250
monster.outfit = {
	lookType = 304,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9801
monster.health = 1470
monster.maxHealth = 1470
monster.race = "energy"
monster.speed = 160
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
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "INTRUDER ALARM!", yell = false},
	{text = "klonk klonk klonk", yell = false},
	{text = "Rrrtttarrrttarrrtta", yell = false},
	{text = "Awaiting orders.", yell = false},
	{text = "Secret objective complete.", yell = false},
}

monster.loot = {
	{id = 2145, chance = 1000, maxCount = 2}, -- small diamond
	{id = 2148, chance = 43000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 40}, -- gold coin
	{id = 2164, chance = 370}, -- might ring
	{id = 2177, chance = 890}, -- life crystal
	{id = 2391, chance = 920}, -- war hammer
	{id = 5880, chance = 1001}, -- iron ore
	{id = 7428, chance = 130}, -- bonebreaker
	{id = 7439, chance = 820}, -- berserk potion
	{id = 7452, chance = 1003}, -- spiked squelcher
	{id = 7590, chance = 1470}, -- great mana potion
	{id = 7591, chance = 2100}, -- great health potion
	{id = 8309, chance = 5000, maxCount = 5}, -- nail
	{id = 8472, chance = 830}, -- great spirit potion
	{id = 9690, chance = 4007}, -- gear wheel
	{id = 9809, chance = 200},
	{id = 9812, chance = 50},
	{id = 9979, chance = 2270}, -- crystal pedestal
	{id = 10572, chance = 2270}, -- gear crystal
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -240, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -125, range = 7, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 35,
	{name = "combat", interval = 2000, chance = 10, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)