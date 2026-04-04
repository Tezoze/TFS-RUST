local mType = Game.createMonsterType("Crawler")
local monster = {}

monster.description = "a crawler"
monster.experience = 1000
monster.outfit = {
	lookType = 456,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15292
monster.health = 1450
monster.maxHealth = 1450
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
	interval = 5000,
	chance = 10,
	{text = "Sssschrchrsss!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 90}, -- gold coin
	{id = 2154, chance = 530}, -- yellow gem
	{id = 2214, chance = 50000}, -- life ring
	{id = 2391, chance = 2070}, -- war hammer
	{id = 7590, chance = 9300}, -- great mana potion
	{id = 7591, chance = 6200}, -- great health potion
	{id = 8912, chance = 710}, -- springsprout rod
	{id = 9057, chance = 10040, maxCount = 2}, -- small topaz
	{id = 15477, chance = 18430}, -- crawler head plating
	{id = 15486, chance = 14640}, -- compound eye
	{id = 15490, chance = 100}, -- grasshopper legs
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -228, target = false, condition = {type = CONDITION_POISON, startDamage = 80, interval = 4000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -100, maxDamage = -180, shootEffect = CONST_ANI_SMALLEARTH, target = true, range = 7, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "speed", interval = 2000, chance = 15, speed = 300, effect = CONST_ME_MAGIC_RED, target = false, duration = 3000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
