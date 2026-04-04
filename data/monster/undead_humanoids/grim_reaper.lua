local mType = Game.createMonsterType("Grim Reaper")
local monster = {}

monster.description = "a grim reaper"
monster.experience = 5500
monster.outfit = {
	lookType = 300,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8955
monster.health = 3900
monster.maxHealth = 3900
monster.race = "undead"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 20
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
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Death!", yell = false},
	{text = "Come a little closer!", yell = false},
	{text = "The end is near!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 99000, maxCount = 263}, -- gold coin
	{id = 2152, chance = 5200, maxCount = 4}, -- platinum coin
	{id = 2162, chance = 4850}, -- magic light wand
	{id = 2521, chance = 3000}, -- dark shield
	{id = 2550, chance = 9000}, -- scythe
	{id = 5022, chance = 1400, maxCount = 4}, -- orichalcum pearl
	{id = 6300, chance = 330}, -- death ring
	{id = 6500, chance = 10600}, -- demonic essence
	{id = 6558, chance = 35000}, -- concentrated demonic blood
	{id = 7418, chance = 880}, -- nightmare blade
	{id = 7590, chance = 10000}, -- great mana potion
	{id = 7896, chance = 330}, -- glacier kilt
	{id = 8473, chance = 9600}, -- ultimate health potion
	{id = 8889, chance = 270}, -- skullcracker armor
	{id = 8910, chance = 910}, -- underworld rod
	{id = 9810, chance = 2500},
	{id = 10577, chance = 5300}, -- mystical hourglass
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -320, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -165, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -350, maxDamage = -720, effect = CONST_ME_REDSPARK, target = false, length = 8, spread = 0, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -300, effect = CONST_ME_EXPLOSIONAREA, target = false, length = 7, spread = 3, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -225, maxDamage = -275, radius = 4, effect = CONST_ME_REDSPARK, target = false, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 30,
	{name = "combat", interval = 2000, chance = 15, minDamage = 130, maxDamage = 205, effect = CONST_ME_REDSPARK, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 450, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = 40},
	{type = COMBAT_ICEDAMAGE, percent = 65},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 80},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)