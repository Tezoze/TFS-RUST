local mType = Game.createMonsterType("Nightstalker")
local monster = {}

monster.description = "a nightstalker"
monster.experience = 500
monster.outfit = {
	lookType = 320,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9915
monster.health = 700
monster.maxHealth = 700
monster.race = "undead"
monster.speed = 260
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 0,
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
	{text = "The sunlight is so depressing.", yell = false},
	{text = "Come with me, my child.", yell = false},
	{text = "I've been in the shadow under your bed last night.", yell = false},
	{text = "You never know what hides in the night.", yell = false},
	{text = "I remember your face - and I know where you sleep.", yell = false},
	{text = "Only the sweetest and cruelest dreams for you, my love.", yell = false},
}

monster.loot = {
	{id = 2124, chance = 1030}, -- crystal ring
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 10}, -- gold coin
	{id = 2171, chance = 121}, -- platinum amulet
	{id = 2195, chance = 121}, -- boots of haste
	{id = 2200, chance = 847}, -- protection amulet
	{id = 2804, chance = 4761}, -- shadow herb
	{id = 7407, chance = 318}, -- haunted blade
	{id = 7427, chance = 121}, -- chaos mace
	{id = 7589, chance = 1612}, -- strong mana potion
	{id = 8870, chance = 520}, -- spirit cloak
	{id = 9942, chance = 127}, -- crystal of balance
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false, condition = {type = CONDITION_POISON, startDamage = 80, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -60, maxDamage = -170, range = 7, effect = CONST_ME_HOLYAREA, target = true, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_SLEEP, target = true, speed = -600, duration = 15000},
}

monster.defenses = {
	defense = 15,
	armor = 40,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 240, duration = 5000},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_YELLOWBUBBLE},
	{name = "outfit", interval = 5000, chance = 10, monster = "nightstalker", duration = 4000},
	{name = "outfit", interval = 5000, chance = 10, monster = "werewolf", duration = 4000},
	{name = "outfit", interval = 5000, chance = 10, monster = "the count", duration = 4000},
	{name = "outfit", interval = 5000, chance = 10, monster = "grim reaper", duration = 4000},
	{name = "outfit", interval = 5000, chance = 10, monster = "tarantula", duration = 4000},
	{name = "outfit", interval = 5000, chance = 1, monster = "ferumbras", duration = 4000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)