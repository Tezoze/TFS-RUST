local mType = Game.createMonsterType("YalahariMonster")
local monster = {}

monster.description = "a yalahari monster"
monster.experience = 150
monster.outfit = {
	lookType = 310,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6068
monster.health = 300
monster.maxHealth = 300
monster.race = "blood"
monster.speed = 220
monster.manaCost = 600
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
	{text = "For the Yalahari!", yell = false},
	{text = "Your end is near!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 50}, -- gold coin
	{id = 9778, chance = 500}, -- yalahari mask
	{id = 9776, chance = 300}, -- yalahari armor
	{id = 2147, chance = 1000}, -- small ruby
	{id = 7588, chance = 1000}, -- strong health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)