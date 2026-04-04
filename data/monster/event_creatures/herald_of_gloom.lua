local mType = Game.createMonsterType("Herald of Gloom")
local monster = {}

monster.description = "a herald of gloom"
monster.experience = 450
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
monster.health = 340
monster.maxHealth = 340
monster.race = "undead"
monster.speed = 170
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
	{text = "The powers of light are waning.", yell = true},
	{text = "You will join us in eternal night!", yell = true},
	{text = "The shadows will engulf the world.", yell = true},
}

monster.loot = {
	{id = 10531, chance = 1886}, -- midnight shard
}

monster.attacks = {
	{name = "melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -90, target = false},
	{name = "speed", interval = 3000, chance = 10, range = 7, effect = CONST_ME_REDSHIMMER, target = true, speed = -600, duration = 30000},
	{name = "combat", interval = 2000, chance = 24, minDamage = -90, maxDamage = -170, range = 4, shootEffect = CONST_ANI_SMALLHOLY, target = true, type = COMBAT_HOLYDAMAGE},
}

monster.defenses = {
	defense = 55,
	armor = 25,
	{name = "speed", interval = 1000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 200, duration = 20000},
	{name = "invisible", interval = 5000, chance = 20, effect = CONST_ME_REDSHIMMER},
	{name = "outfit", interval = 1500, chance = 20, effect = CONST_ME_BLUESHIMMER, monster = "nightstalker", duration = 6000},
	{name = "outfit", interval = 1500, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "werewolf", duration = 6000},
	{name = "outfit", interval = 1500, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "the count", duration = 6000},
	{name = "outfit", interval = 1500, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "grim reaper", duration = 6000},
	{name = "outfit", interval = 1500, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "tarantula", duration = 6000},
	{name = "outfit", interval = 1500, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "ferumbras", duration = 6000},
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