local mType = Game.createMonsterType("Diblis The Fair")
local monster = {}

monster.description = "Diblis The Fair"
monster.experience = 1800
monster.outfit = {
	lookType = 287,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8937
monster.health = 1500
monster.maxHealth = 1500
monster.race = "undead"
monster.speed = 280
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 5000,
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I envy you to be slain by someone as beautiful as me.", yell = false},
	{text = "I will drain your ugly corpses of the last drop of blood.", yell = false},
	{text = "Not in my face you barbarian!", yell = false},
	{text = "My brides will feast on your souls!", yell = false},
}

monster.loot = {
	{id = 2229, chance = 15000}, -- skull
	{id = 7588, chance = 1500}, -- strong health potion
	{id = 2144, chance = 8900, maxCount = 2}, -- black pearl
	{id = 2152, chance = 50000, maxCount = 5}, -- platinum coin
	{id = 2148, chance = 100000, maxCount = 99}, -- gold coin
	{id = 9020, chance = 100000}, -- vampire lord token
	{id = 2534, chance = 2100}, -- vampire shield
	{id = 8903, chance = 300}, -- spellbook of lost souls
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 70, attack = 95, target = false},
	{name = "combat", interval = 1000, chance = 12, minDamage = 0, maxDamage = -155, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 1000, chance = 12, minDamage = 100, maxDamage = 235, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 3000, chance = 25, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -15},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Banshee", chance = 50, interval = 4500, max = 4},
}

mType:register(monster)