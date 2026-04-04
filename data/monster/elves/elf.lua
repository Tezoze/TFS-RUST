local mType = Game.createMonsterType("Elf")
local monster = {}

monster.description = "an elf"
monster.experience = 42
monster.outfit = {
	lookType = 62,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6003
monster.health = 100
monster.maxHealth = 100
monster.race = "blood"
monster.speed = 190
monster.manaCost = 320
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
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ulathil beia Thratha!", yell = false},
	{text = "Bahaha aka!", yell = false},
	{text = "You are not welcome here.", yell = false},
	{text = "Flee as long as you can.", yell = false},
	{text = "Death to the Defilers!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 44000, maxCount = 30}, -- gold coin
	{id = 2397, chance = 10800}, -- longsword
	{id = 2482, chance = 13500}, -- studded helmet
	{id = 2484, chance = 8960}, -- studded armor
	{id = 2510, chance = 9300}, -- plate shield
	{id = 2544, chance = 7060, maxCount = 3}, -- arrow
	{id = 2643, chance = 11410}, -- leather boots
	{id = 5921, chance = 940}, -- heaven blossom
	{id = 8839, chance = 20000, maxCount = 2}, -- plum
	{id = 10552, chance = 2100}, -- elvish talisman
	{id = 25360, chance = 4500}, -- botanist's container
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -15, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -25, range = 7, shootEffect = CONST_ANI_ARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 6,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)