local mType = Game.createMonsterType("Necromancer")
local monster = {}

monster.description = "a necromancer"
monster.experience = 580
monster.outfit = {
	lookType = 9,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 580
monster.maxHealth = 580
monster.race = "blood"
monster.speed = 188
monster.manaCost = 0
monster.maxSummons = 2

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
	canPushCreatures = true,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Your corpse will be mine.", yell = false},
	{text = "Taste the sweetness of death!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 30050, maxCount = 90}, -- gold coin
	{id = 2195, chance = 210}, -- boots of haste
	{id = 2423, chance = 390}, -- clerical mace
	{id = 2436, chance = 100}, -- skull staff
	{id = 2545, chance = 15000, maxCount = 5}, -- poison arrow
	{id = 2663, chance = 500}, -- mystic turban
	{id = 2796, chance = 1470}, -- green mushroom
	{id = 7456, chance = 10}, -- noble axe
	{id = 7589, chance = 300}, -- strong mana potion
	{id = 8901, chance = 130}, -- spellbook of warding
	{id = 11237, chance = 10130}, -- book of necromantic rituals
	{id = 12431, chance = 1001}, -- necromantic robe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false, condition = {type = CONDITION_POISON, startDamage = 160, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -60, maxDamage = -120, range = 1, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -65, maxDamage = -120, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 50,
	{name = "combat", interval = 2000, chance = 25, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}

monster.summons = {
	{name = "Ghoul", chance = 15, interval = 2000, max = 2},
	{name = "Ghost", chance = 5, interval = 2000, max = 2},
	{name = "Mummy", chance = 5, interval = 2000, max = 2},
}

mType:register(monster)