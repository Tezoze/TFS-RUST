local mType = Game.createMonsterType("Draken Elite")
local monster = {}

monster.description = "a draken elite"
monster.experience = 4200
monster.outfit = {
	lookType = 362,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 12609
monster.health = 5550
monster.maxHealth = 5550
monster.race = "blood"
monster.speed = 332
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "For ze emperor!", yell = false},
	{text = "You will die zhouzandz deazhz!", yell = false},
}

monster.loot = {
	{id = 2145, chance = 2440, maxCount = 4}, -- small diamond
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 47000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 50360, maxCount = 8}, -- platinum coin
	{id = 2666, chance = 30175}, -- meat
	{id = 5904, chance = 2100}, -- magic sulphur
	{id = 7404, chance = 980}, -- assassin dagger
	{id = 7590, chance = 9340, maxCount = 3}, -- great mana potion
	{id = 8473, chance = 9250, maxCount = 3}, -- ultimate health potion
	{id = 11301, chance = 490}, -- Zaoan armor
	{id = 11302, chance = 150}, -- Zaoan helmet
	{id = 11304, chance = 770}, -- Zaoan legs
	{id = 11307, chance = 490}, -- Zaoan sword
	{id = 12607, chance = 110}, -- elite draken mail
	{id = 12613, chance = 910}, -- twiceslicer
	{id = 12614, chance = 7600}, -- draken sulphur
	{id = 12615, chance = 14030}, -- draken wristbands
	{id = 12616, chance = 16930}, -- broken draken mail
	{id = 12617, chance = 24670}, -- broken slicer
	{id = 12630, chance = 10}, -- cobra crown
	{id = 12646, chance = 600}, -- draken boots
	{id = 12647, chance = 80}, -- snake god's wristguard
	{id = 12649, chance = 20}, -- blade of corruption
	{id = 11134, chance = 1000}, -- Tome of Knowledge
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -354, interval = 2000, target = false},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -240, maxDamage = -550, interval = 2000, chance = 10, length = 4, spread = 3, target = false, effect = CONST_ME_EXPLOSION},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -200, maxDamage = -300, interval = 2000, chance = 15, range = 7, target = true, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -280, maxDamage = -410, interval = 2000, chance = 15, radius = 4, target = true, effect = CONST_ME_POFF},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -250, maxDamage = -320, range = 7, shootEffect = CONST_ANI_POISON, target = true},
}

monster.defenses = {
	defense = 60,
	armor = 60,
	{name = "combat", interval = 2000, chance = 15, minDamage = 510, maxDamage = 600, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 40},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)