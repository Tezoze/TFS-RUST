local mType = Game.createMonsterType("Roaring Water Elemental")
local monster = {}

monster.description = "a roaring water elemental"
monster.experience = 1300
monster.outfit = {
	lookType = 11,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8965
monster.health = 1750
monster.maxHealth = 1750
monster.race = "undead"
monster.speed = 390
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 20000,
	chance = 15
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 85,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "BLUB BLUB", yell = false},
	{text = "SWASHHH", yell = false},
}

monster.loot = {
	{id = 2146, chance = 4125, maxCount = 2}, -- small sapphire
	{id = 2148, chance = 27000, maxCount = 90}, -- gold coin
	{id = 2148, chance = 27000, maxCount = 87}, -- gold coin
	{id = 8302, chance = 9000}, -- iced soil
	{id = 8911, chance = 750}, -- northwind rod
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -225, target = false},
	{name = "combat", interval = 1000, chance = 15, minDamage = -240, maxDamage = -320, radius = 2, shootEffect = CONST_ANI_ICE, effect = CONST_ME_BLUEBUBBLE, target = true, type = COMBAT_ICEDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 15, minDamage = 90, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)