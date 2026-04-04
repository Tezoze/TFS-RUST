local mType = Game.createMonsterType("Diabolic Imp")
local monster = {}

monster.description = "a diabolic imp"
monster.experience = 2900
monster.outfit = {
	lookType = 237,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6364
monster.health = 1950
monster.maxHealth = 1950
monster.race = "fire"
monster.speed = 210
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 400,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Muahaha!", yell = false},
	{text = "He he he.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 97}, -- gold coin
	{id = 2148, chance = 3390, maxCount = 7}, -- gold coin
	{id = 2150, chance = 2250, maxCount = 3}, -- small amethyst
	{id = 2165, chance = 2702}, -- stealth ring
	{id = 2185, chance = 830}, -- necrotic rod
	{id = 2260, chance = 16666, maxCount = 2}, -- blank rune
	{id = 2387, chance = 1994}, -- double axe
	{id = 2419, chance = 5660}, -- scimitar
	{id = 2515, chance = 8130}, -- guardian shield
	{id = 2548, chance = 50000}, -- pitchfork
	{id = 2568, chance = 8830}, -- cleaver
	{id = 5944, chance = 7230}, -- soul orb
	{id = 6300, chance = 120}, -- death ring
	{id = 6500, chance = 8000}, -- demonic essence
	{id = 6558, chance = 25000, maxCount = 2}, -- concentrated demonic blood
	{id = 7899, chance = 250}, -- magma coat
	{id = 7900, chance = 430}, -- magma monocle
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -240, target = false, condition = {type = CONDITION_POISON, startDamage = 160, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -100, maxDamage = -240, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -300, maxDamage = -430, range = 7, radius = 2, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREATTACK, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 5, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 29,
	{name = "combat", interval = 2000, chance = 10, minDamage = 650, maxDamage = 800, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 800, duration = 2000},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_TELEPORT},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)