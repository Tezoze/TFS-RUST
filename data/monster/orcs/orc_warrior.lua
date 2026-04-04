local mType = Game.createMonsterType("Orc Warrior")
local monster = {}

monster.description = "an orc warrior"
monster.experience = 50
monster.outfit = {
	lookType = 7,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5979
monster.health = 125
monster.maxHealth = 125
monster.race = "blood"
monster.speed = 190
monster.manaCost = 360
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 11,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Grow truk grrrr.", yell = false},
	{text = "Trak grrrr brik.", yell = false},
	{text = "Alk!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 65000, maxCount = 15}, -- gold coin
	{id = 2411, chance = 120}, -- poison dagger
	{id = 2464, chance = 7360}, -- chain armor
	{id = 2530, chance = 560}, -- copper shield
	{id = 2666, chance = 15000}, -- meat
	{id = 11113, chance = 700}, -- orc tooth
	{id = 12409, chance = 10800}, -- broken helmet
	{id = 12435, chance = 4000}, -- orc leather
	{id = 12436, chance = 980}, -- skull belt
	{id = 1950, chance = 1000}, -- book
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)