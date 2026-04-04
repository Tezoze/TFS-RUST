local mType = Game.createMonsterType("Deepling Brawler")
local monster = {}

monster.description = "a deepling brawler"
monster.experience = 260
monster.outfit = {
	lookType = 470,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13840
monster.health = 380
monster.maxHealth = 380
monster.race = "blood"
monster.speed = 190
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 40,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.loot = {
	{id = 2148, chance = 61000, maxCount = 44}, -- gold coin
	{id = 2667, chance = 19120, maxCount = 3}, -- fish
	{id = 5895, chance = 740}, -- fish fin
	{id = 13838, chance = 2940}, -- heavy trident
	{id = 13870, chance = 6600}, -- eye of a deepling
	{id = 15430, chance = 14500}, -- deepling scales
	{id = 2149, chance = 500}, -- small emerald
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -120, shootEffect = CONST_ANI_SPEAR, effect = CONST_ME_BLUE_BUBBLE, target = true, range = 7, type = COMBAT_DROWNDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)
