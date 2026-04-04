local mType = Game.createMonsterType("Dwarf Guard")
local monster = {}

monster.description = "a dwarf guard"
monster.experience = 165
monster.outfit = {
	lookType = 70,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6013
monster.health = 245
monster.maxHealth = 245
monster.race = "blood"
monster.speed = 206
monster.manaCost = 650
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hail Durin!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 40000, maxCount = 30}, -- gold coin
	{id = 2150, chance = 140}, -- small amethyst
	{id = 2207, chance = 190}, -- melee ring
	{id = 2387, chance = 600}, -- double axe
	{id = 2417, chance = 4000}, -- battle hammer
	{id = 2457, chance = 1600}, -- steel helmet
	{id = 2483, chance = 9200}, -- scale armor
	{id = 2513, chance = 6000}, -- battle shield
	{id = 2643, chance = 40000}, -- leather boots
	{id = 2787, chance = 55000, maxCount = 2}, -- white mushroom
	{id = 5880, chance = 3510}, -- iron ore
	{id = 7618, chance = 380}, -- health potion
	{id = 8748, chance = 280},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -140, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)