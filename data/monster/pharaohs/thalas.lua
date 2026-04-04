local mType = Game.createMonsterType("Thalas")
local monster = {}

monster.description = "Thalas"
monster.experience = 2950
monster.outfit = {
	lookType = 90,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6025
monster.health = 4100
monster.maxHealth = 4100
monster.race = "undead"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 8

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	interval = 5000,
	chance = 10,
	{text = "You will become a feast for my maggots!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 80}, -- gold coin
	{id = 2149, chance = 7000, maxCount = 3}, -- small emerald
	{id = 2155, chance = 500}, -- green gem
	{id = 2165, chance = 1500}, -- stealth ring
	{id = 2169, chance = 7000}, -- time ring
	{id = 2351, chance = 100000}, -- cobrafang dagger
	{id = 2409, chance = 500}, -- serpent sword
	{id = 2411, chance = 7000}, -- poison dagger
	{id = 2451, chance = 200}, -- djinn blade
	{id = 7591, chance = 1500}, -- great health potion
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -900, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -150, maxDamage = -650, interval = 2000, chance = 25, range = 7, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON},
	{name = "melee", minDamage = -150, maxDamage = -650, interval = 3000, chance = 20, range = 7, radius = 1, target = true, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON},
	{name = "speed", interval = 1000, chance = 6, range = 7, target = true, effect = CONST_ME_REDSHIMMER, speed = -800, duration = 20000},
	{name = "condition", type = CONDITION_POISON, interval = 1000, chance = 15, tick = 4000, minDamage = -34, maxDamage = -35, radius = 5, effect = CONST_ME_POISON, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -55, maxDamage = -550, interval = 3000, chance = 17, length = 8, spread = 3, target = false, effect = CONST_ME_POISON},
}

monster.defenses = {
	defense = 30,
	armor = 20,
	{name = "combat", interval = 1000, chance = 20, minDamage = 150, maxDamage = 450, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -23},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Slime", chance = 100, interval = 2000, max = 8},
}

mType:register(monster)