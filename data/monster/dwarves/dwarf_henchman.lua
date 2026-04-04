local mType = Game.createMonsterType("Dwarf Henchman")
local monster = {}

monster.description = "a dwarf henchman"
monster.experience = 15
monster.outfit = {
	lookType = 160,
	lookHead = 115,
	lookBody = 77,
	lookLegs = 112,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6007
monster.health = 350
monster.maxHealth = 350
monster.race = "blood"
monster.speed = 170
monster.manaCost = 0
monster.maxSummons = 0

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
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
	{text = "This place is for our eyes only!", yell = false},
	{text = "We will live and let you die!", yell = false},
	{text = "I will die another day!", yell = false},
	{text = "We have license to kill!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 40000, maxCount = 30}, -- gold coin
	{id = 2150, chance = 140}, -- small amethyst
	{id = 2207, chance = 190}, -- melee ring
	{id = 5880, chance = 3510}, -- iron ore
	{id = 2787, chance = 55000, maxCount = 2}, -- white mushroom
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -50, interval = 2000, target = false},
	{name = "condition", type = CONDITION_DROWN, interval = 2000, chance = 20, tick = 5000, minDamage = -80, maxDamage = -80, range = 7, duration = 20000, target = true},
}

monster.defenses = {
	defense = 10,
	armor = 16,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_ICEDAMAGE, percent = 15},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = 15},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)