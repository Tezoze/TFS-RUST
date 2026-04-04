local mType = Game.createMonsterType("Dwarf Soldier")
local monster = {}

monster.description = "a dwarf soldier"
monster.experience = 70
monster.outfit = {
	lookType = 71,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6014
monster.health = 135
monster.maxHealth = 135
monster.race = "blood"
monster.speed = 176
monster.manaCost = 360
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
	canPushCreatures = false,
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
	{id = 2148, chance = 28000, maxCount = 12}, -- gold coin
	{id = 2207, chance = 120}, -- melee ring
	{id = 2378, chance = 2500}, -- battle axe
	{id = 2455, chance = 3000}, -- crossbow
	{id = 2464, chance = 8000}, -- chain armor
	{id = 2481, chance = 12000}, -- soldier helmet
	{id = 2525, chance = 3000}, -- dwarven shield
	{id = 2543, chance = 40000, maxCount = 7}, -- bolt
	{id = 2554, chance = 10000}, -- shovel
	{id = 2787, chance = 40000, maxCount = 3}, -- white mushroom
	{id = 5880, chance = 3300}, -- iron ore
	{id = 7363, chance = 4000, maxCount = 3}, -- piercing bolt
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -60, range = 7, shootEffect = CONST_ANI_BOLT, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 9,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)