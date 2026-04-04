local mType = Game.createMonsterType("Lizard Magistratus")
local monster = {}

monster.description = "a lizard magistratus"
monster.experience = 2000
monster.outfit = {
	lookType = 115,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6041
monster.health = 8000
monster.maxHealth = 8000
monster.race = "blood"
monster.speed = 256
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Shhhhhhhh.", yell = false},
	{text = "I can't work wizh zuch dizturbancez!", yell = false},
}

monster.loot = {
	{id = 2147, chance = 8970, maxCount = 5}, -- small ruby
	{id = 2148, chance = 77230, maxCount = 50}, -- gold coin
	{id = 2152, chance = 13400, maxCount = 19}, -- platinum coin
	{id = 5876, chance = 220}, -- lizard leather
	{id = 5881, chance = 3450}, -- lizard scale
	{id = 7589, chance = 6280}, -- strong mana potion
	{id = 7590, chance = 4480}, -- great mana potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
	{name = "combat", interval = 2000, chance = 10, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 25,
	{name = "combat", interval = 2000, chance = 50, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 80},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)