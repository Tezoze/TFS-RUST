local mType = Game.createMonsterType("Lizard Templar")
local monster = {}

monster.description = "a lizard templar"
monster.experience = 155
monster.outfit = {
	lookType = 113,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 4251
monster.health = 410
monster.maxHealth = 410
monster.race = "blood"
monster.speed = 174
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hissss!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 84000, maxCount = 60}, -- gold coin
	{id = 2149, chance = 250}, -- small emerald
	{id = 2376, chance = 4000}, -- sword
	{id = 2394, chance = 1990}, -- morning star
	{id = 2406, chance = 9500}, -- short sword
	{id = 2457, chance = 2000}, -- steel helmet
	{id = 2463, chance = 1000}, -- plate armor
	{id = 3963, chance = 500}, -- templar scytheblade
	{id = 3975, chance = 110}, -- salamander shield
	{id = 5876, chance = 2880}, -- lizard leather
	{id = 5881, chance = 3990}, -- lizard scale
	{id = 7618, chance = 890}, -- health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
}

monster.defenses = {
	defense = 20,
	armor = 26,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)