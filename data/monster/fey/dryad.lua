local mType = Game.createMonsterType("Dryad")
local monster = {}

monster.description = "a dryad"
monster.experience = 190
monster.outfit = {
	lookType = 137,
	lookHead = 80,
	lookBody = 59,
	lookLegs = 7,
	lookFeet = 101,
	lookAddons = 3,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 310
monster.maxHealth = 310
monster.race = "blood"
monster.speed = 230
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
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 10,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Feel the wrath of mother Tibia!", yell = false},
	{text = "Defiler of nature!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 40000, maxCount = 20}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -40, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}


mType:register(monster)