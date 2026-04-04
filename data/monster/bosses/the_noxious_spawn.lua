local mType = Game.createMonsterType("The Noxious Spawn")
local monster = {}

monster.description = "The Noxious Spawn"
monster.experience = 6000
monster.outfit = {
	lookType = 220,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 4323
monster.health = 9500
monster.maxHealth = 9500
monster.race = "venom"
monster.speed = 360
monster.manaCost = 0
monster.maxSummons = 0

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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 275,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I bring you deathhhh, mortalssss", yell = false},
}

monster.loot = {
	{id = 10611, chance = 100000}, -- snake skin
	{id = 11230, chance = 100000}, -- winged tail
	{id = 2152, chance = 80000, maxCount = 5}, -- platinum coin
	{id = 7590, chance = 72000, maxCount = 4}, -- great mana potion
	{id = 2149, chance = 68000, maxCount = 5}, -- small emerald
	{id = 7386, chance = 45000}, -- mercenary sword
	{id = 2528, chance = 43000}, -- tower shield
	{id = 7456, chance = 39000}, -- noble axe
	{id = 2033, chance = 35000}, -- golden mug
	{id = 2487, chance = 29000}, -- crown armor
	{id = 7368, chance = 27000, maxCount = 78}, -- assassin star
	{id = 2796, chance = 19000}, -- green mushroom
	{id = 2168, chance = 13000}, -- life ring
	{id = 8902, chance = 13000}, -- spellbook of mind control
	{id = 2498, chance = 4000}, -- royal helmet
	{id = 8880, chance = 2000}, -- swamplair armor
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
	{name = "speed", interval = 4000, chance = 20, range = 7, shootEffect = CONST_ANI_POISON, target = true, speed = -370, duration = 12000},
	{name = "combat", interval = 2000, chance = 7, minDamage = 0, maxDamage = -550, effect = CONST_ME_POISON, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 9, minDamage = 0, maxDamage = -550, effect = CONST_ME_REDNOTE, target = false, length = 8, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 12, minDamage = 0, maxDamage = -300, range = 1, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "outfit", interval = 2000, chance = 11, range = 7, effect = CONST_ME_BLUESHIMMER, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 18,
	{name = "combat", interval = 2000, chance = 10, minDamage = 900, maxDamage = 1000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)