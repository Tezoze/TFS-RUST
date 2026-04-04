local mType = Game.createMonsterType("Lord of the Elements")
local monster = {}

monster.description = "Lord of the Elements"
monster.experience = 8000
monster.outfit = {
	lookType = 290,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9009
monster.health = 8000
monster.maxHealth = 8000
monster.race = "undead"
monster.speed = 370
monster.manaCost = 0
monster.maxSummons = 2

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
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 30,
	{text = "WHO DARES CALLING ME?", yell = true},
	{text = "I'LL FREEZE YOU THEN I CRUSH YOU!", yell = true},
}

monster.loot = {
	{id = 2146, chance = 7142, maxCount = 4}, -- small sapphire
	{id = 2147, chance = 11111, maxCount = 4}, -- small ruby
	{id = 2149, chance = 11111, maxCount = 4}, -- small emerald
	{id = 2150, chance = 11111, maxCount = 3}, -- small amethyst
	{id = 2152, chance = 50000, maxCount = 9}, -- platinum coin
	{id = 8882, chance = 2063}, -- earthborn titan armor
	{id = 9971, chance = 25000}, -- gold ingot
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -690, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 1000, chance = 25, minDamage = 100, maxDamage = 195, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "outfit", interval = 1500, chance = 40, effect = CONST_ME_BLUESHIMMER, monster = "Energy Overlord", duration = 3000},
	{name = "outfit", interval = 1500, chance = 40, effect = CONST_ME_BLUESHIMMER, monster = "Fire Overlord", duration = 3000},
	{name = "outfit", interval = 1500, chance = 40, effect = CONST_ME_BLUESHIMMER, monster = "Earth Overlord", duration = 3000},
	{name = "outfit", interval = 1500, chance = 40, effect = CONST_ME_BLUESHIMMER, monster = "Ice Overlord", duration = 3000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 1},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 45},
}

monster.immunities = {
	{type = "holy", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Blistering Fire Elemental", chance = 50, interval = 4000, max = 2},
	{name = "Jagged Earth Elemental", chance = 50, interval = 4000, max = 2},
	{name = "Roaring Water Elemental", chance = 50, interval = 4000, max = 2},
	{name = "Overcharged Energy Elemental", chance = 50, interval = 4000, max = 2},
}

mType:register(monster)