local mType = Game.createMonsterType("Quara Predator Scout")
local monster = {}

monster.description = "a quara predator scout"
monster.experience = 400
monster.outfit = {
	lookType = 20,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6067
monster.health = 890
monster.maxHealth = 890
monster.race = "blood"
monster.speed = 170
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Gnarrr!", yell = false},
	{text = "Tcharrr!", yell = false},
	{text = "Rrrah!", yell = false},
	{text = "Rraaar!", yell = false},
}

monster.loot = {
	{id = 2145, chance = 5520, maxCount = 2}, -- small diamond
	{id = 2148, chance = 50000, maxCount = 60}, -- gold coin
	{id = 2148, chance = 48000, maxCount = 69}, -- gold coin
	{id = 2377, chance = 2320}, -- two handed sword
	{id = 2387, chance = 5800}, -- double axe
	{id = 2483, chance = 8000}, -- scale armor
	{id = 2670, chance = 4700, maxCount = 5}, -- shrimp
	{id = 5895, chance = 5940, maxCount = 2}, -- fish fin
	{id = 8911, chance = 800}, -- northwind rod
	{id = 12447, chance = 10150}, -- quara bone
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -193, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)