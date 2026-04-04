local mType = Game.createMonsterType("Elf Scout")
local monster = {}

monster.description = "an elf scout"
monster.experience = 75
monster.outfit = {
	lookType = 64,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6012
monster.health = 160
monster.maxHealth = 160
monster.race = "blood"
monster.speed = 220
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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Tha'shi Ab'Dendriel!", yell = false},
	{text = "Feel the sting of my arrows!", yell = false},
	{text = "Thy blood will quench the soil's thirst!", yell = false},
	{text = "Evicor guide my arrow!", yell = false},
	{text = "Your existence will end here!", yell = false},
}

monster.loot = {
	{id = 2031, chance = 1350}, -- waterskin
	{id = 2148, chance = 75000, maxCount = 25}, -- gold coin
	{id = 2456, chance = 4000}, -- bow
	{id = 2544, chance = 30710, maxCount = 12}, -- arrow
	{id = 2545, chance = 15400, maxCount = 4}, -- poison arrow
	{id = 2642, chance = 1180}, -- sandals
	{id = 2681, chance = 17750}, -- grapes
	{id = 5921, chance = 1130}, -- heaven blossom
	{id = 7438, chance = 140}, -- elvish bow
	{id = 10552, chance = 5200}, -- elvish talisman
	{id = 12420, chance = 9750}, -- elven scouting glass
	{id = 25360, chance = 1500}, -- botanist's container
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -80, range = 7, shootEffect = CONST_ANI_ARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 7,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)