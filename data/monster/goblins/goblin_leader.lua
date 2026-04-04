local mType = Game.createMonsterType("Goblin Leader")
local monster = {}

monster.description = "a goblin leader"
monster.experience = 75
monster.outfit = {
	lookType = 61,
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
monster.speed = 120
monster.manaCost = 290
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
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Go go, Gobo attack!!", yell = false},
	{text = "Me the greenest and the meanest!", yell = false},
	{text = "Me have power to crush you!", yell = false},
	{text = "Goblinkiller! Catch him !!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 40000, maxCount = 10}, -- gold coin
	{id = 2230, chance = 11500}, -- bone
	{id = 2235, chance = 9000}, -- mouldy cheese
	{id = 2379, chance = 10300}, -- dagger
	{id = 2406, chance = 15400}, -- short sword
	{id = 2449, chance = 1300}, -- bone club
	{id = 2461, chance = 16670}, -- leather helmet
	{id = 2467, chance = 5000}, -- leather armor
	{id = 2559, chance = 12800}, -- small axe
	{id = 2667, chance = 15000}, -- fish
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -45, range = 7, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 7,
}


mType:register(monster)