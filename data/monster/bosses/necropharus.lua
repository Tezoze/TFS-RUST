local mType = Game.createMonsterType("Necropharus")
local monster = {}

monster.description = "Necropharus"
monster.experience = 1050
monster.outfit = {
	lookType = 209,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 750
monster.maxHealth = 750
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "You will rise as my servant!", yell = false},
	{text = "Praise to my master Urgith!", yell = false},
}

monster.loot = {
	{id = 11237, chance = 100000}, -- book of necromantic rituals
	{id = 2148, chance = 100000, maxCount = 99}, -- gold coin
	{id = 12431, chance = 100000}, -- necromantic robe
	{id = 5809, chance = 100000}, -- soul stone
	{id = 2423, chance = 52000}, -- clerical mace
	{id = 2436, chance = 47000}, -- skull staff
	{id = 2449, chance = 38000}, -- bone club
	{id = 2229, chance = 19000}, -- skull
	{id = 2796, chance = 14000}, -- green mushroom
	{id = 2186, chance = 14000}, -- moonlight rod
	{id = 2231, chance = 9500}, -- big bone
	{id = 2541, chance = 9500}, -- bone shield
	{id = 2195, chance = 4700}, -- boots of haste
	{id = 2663, chance = 4700}, -- mystic turban
	{id = 7589, chance = 4700}, -- strong mana potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false, condition = {type = CONDITION_POISON, startDamage = 8, interval = 2000}},
	{name = "combat", interval = 3000, chance = 70, minDamage = -60, maxDamage = -217, range = 5, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 20, minDamage = -80, maxDamage = -120, range = 1, effect = CONST_ME_REDSPARK, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 1000, chance = 17, minDamage = -50, maxDamage = -140, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 17, minDamage = -50, maxDamage = -140, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = 300, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "poison", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Ghoul", chance = 20, interval = 1000, max = 2},
	{name = "Ghost", chance = 17, interval = 1000, max = 2},
	{name = "Mummy", chance = 15, interval = 1000, max = 2},
}

mType:register(monster)