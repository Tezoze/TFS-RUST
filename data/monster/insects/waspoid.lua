local mType = Game.createMonsterType("Waspoid")
local monster = {}

monster.description = "a waspoid"
monster.experience = 830
monster.outfit = {
	lookType = 462,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15396
monster.health = 1100
monster.maxHealth = 1100
monster.race = "venom"
monster.speed = 210
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
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Peeex!", yell = false},
}

monster.loot = {
	{id = 2127, chance = 2040}, -- emerald bangle
	{id = 2144, chance = 4230}, -- black pearl
	{id = 2148, chance = 40000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 35}, -- gold coin
	{id = 2152, chance = 40430}, -- platinum coin
	{id = 2154, chance = 1040}, -- yellow gem
	{id = 15483, chance = 9096}, -- waspoid claw
	{id = 15484, chance = 13890}, -- waspoid wing
	{id = 15486, chance = 6060}, -- compound eye
	{id = 15490, chance = 230}, -- grasshopper legs
	{id = 15491, chance = 120}, -- carapace shield
	{id = 15492, chance = 330}, -- hive scythe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -248, target = false, condition = {type = CONDITION_POISON, startDamage = 400, interval = 4000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -110, maxDamage = -180, radius = 3, effect = CONST_ME_POISONAREA, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -80, maxDamage = -100, shootEffect = CONST_ANI_POISON, target = true, range = 7, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_ENERGYDAMAGE, percent = 25},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
