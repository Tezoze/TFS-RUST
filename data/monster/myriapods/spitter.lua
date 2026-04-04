local mType = Game.createMonsterType("Spitter")
local monster = {}

monster.description = "a spitter"
monster.experience = 1100
monster.outfit = {
	lookType = 461,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15392
monster.health = 1500
monster.maxHealth = 1500
monster.race = "venom"
monster.speed = 200
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
	illusionable = true,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 40,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Spotz!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 190}, -- gold coin
	{id = 2152, chance = 75250}, -- platinum coin
	{id = 15481, chance = 18000}, -- spitter nose
	{id = 15486, chance = 15000}, -- compound eye
	{id = 2150, chance = 8100, maxCount = 2}, -- small amethyst
	{id = 7590, chance = 7800}, -- great mana potion
	{id = 2789, chance = 7500, maxCount = 3}, -- brown mushroom
	{id = 7591, chance = 5000}, -- great health potion
	{id = 2169, chance = 2400}, -- time ring
	{id = 7449, chance = 2000}, -- crystal sword
	{id = 7440, chance = 300}, -- mastermind potion
	{id = 2171, chance = 260}, -- platinum amulet
	{id = 15489, chance = 230}, -- calopteryx cape
	{id = 2497, chance = 220}, -- crusader helmet
	{id = 2155, chance = 210}, -- green gem
	{id = 15490, chance = 140}, -- grasshopper legs
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -100, maxDamage = -150, radius = 3, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISONAREA, target = true, range = 7, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 2000, chance = 15, speed = -300, shootEffect = CONST_ANI_POISON, target = true, range = 7, duration = 15000},
	{name = "combat", interval = 2000, chance = 30, minDamage = -12, maxDamage = -12, radius = 5, target = false, type = COMBAT_EARTHDAMAGE, condition = {type = CONDITION_POISON, startDamage = 12, interval = 4000}},
}

monster.defenses = {
	defense = 20,
	armor = 20,
	{name = "speed", interval = 2000, chance = 15, speed = 250, effect = CONST_ME_MAGIC_RED, target = false, duration = 5000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 5},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
