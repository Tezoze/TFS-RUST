local mType = Game.createMonsterType("Demon Goblin")
local monster = {}

monster.description = "a demon goblin"
monster.experience = 25
monster.outfit = {
	lookType = 35,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6002
monster.health = 50
monster.maxHealth = 50
monster.race = "blood"
monster.speed = 150
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 40,
	runHealth = 15,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 1294, chance = 15290, maxCount = 3}, -- small stone
	{id = 2148, chance = 50320, maxCount = 9}, -- gold coin
	{id = 2230, chance = 1130}, -- bone
	{id = 2235, chance = 1000}, -- mouldy cheese
	{id = 2379, chance = 1800}, -- dagger
	{id = 2406, chance = 8870}, -- short sword
	{id = 2449, chance = 4900}, -- bone club
	{id = 2461, chance = 1940}, -- leather helmet
	{id = 2467, chance = 2510}, -- leather armor
	{id = 2559, chance = 9700}, -- small axe
	{id = 2667, chance = 12750}, -- fish
	{id = 12495, chance = 910}, -- goblin ear
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -10, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -25, range = 7, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_EARTHDAMAGE, percent = -12},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)