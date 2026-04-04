local mType = Game.createMonsterType("Pirate Skeleton")
local monster = {}

monster.description = "a pirate skeleton"
monster.experience = 85
monster.outfit = {
	lookType = 195,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6070
monster.health = 190
monster.maxHealth = 190
monster.race = "undead"
monster.speed = 176
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 20,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 48000, maxCount = 25}, -- gold coin
	{id = 2229, chance = 4460}, -- skull
	{id = 2230, chance = 4250}, -- bone
	{id = 2231, chance = 5140}, -- big bone
	{id = 2376, chance = 550}, -- sword
	{id = 2406, chance = 1003}, -- short sword
	{id = 2449, chance = 960}, -- bone club
	{id = 10559, chance = 4730}, -- spooky blue eye
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
}


mType:register(monster)