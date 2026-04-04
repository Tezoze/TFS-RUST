local mType = Game.createMonsterType("Smuggler Baron Silvertoe")
local monster = {}

monster.description = "Smuggler Baron Silvertoe"
monster.experience = 170
monster.outfit = {
	lookType = 134,
	lookHead = 38,
	lookBody = 0,
	lookLegs = 94,
	lookFeet = 95,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 280
monster.maxHealth = 280
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 3

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
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 15,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I will make your death look like an accident!", yell = false},
	{text = "You should not have interferred with my bussiness!", yell = false},
	{text = "Bribes are expensive, murder is cheap!", yell = false},
	{text = "I see some profit in your death!", yell = false},
	{text = "I expect you to die!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 80000, maxCount = 30}, -- gold coin
	{id = 2406, chance = 10000}, -- short sword
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -40, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 15, minDamage = 10, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.summons = {
	{name = "Wild Warrior", chance = 20, interval = 2000, max = 3},
}

mType:register(monster)