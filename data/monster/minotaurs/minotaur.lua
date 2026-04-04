local mType = Game.createMonsterType("Minotaur")
local monster = {}

monster.description = "a minotaur"
monster.experience = 50
monster.outfit = {
	lookType = 25,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5969
monster.health = 100
monster.maxHealth = 100
monster.race = "blood"
monster.speed = 168
monster.manaCost = 330
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
	{text = "Kaplar!", yell = false},
	{text = "Hurr", yell = false},
}

monster.loot = {
	{id = 2148, chance = 66340, maxCount = 25}, -- gold coin
	{id = 2172, chance = 110}, -- bronze amulet
	{id = 2376, chance = 4970}, -- sword
	{id = 2386, chance = 3980}, -- axe
	{id = 2398, chance = 13060}, -- mace
	{id = 2460, chance = 7990}, -- brass helmet
	{id = 2464, chance = 10050}, -- chain armor
	{id = 2510, chance = 20030}, -- plate shield
	{id = 2554, chance = 310}, -- shovel
	{id = 2666, chance = 4970}, -- meat
	{id = 5878, chance = 1010}, -- minotaur leather
	{id = 12428, chance = 2020, maxCount = 2}, -- minotaur horn
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 11,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}


mType:register(monster)