local mType = Game.createMonsterType("Deepling Elite")
local monster = {}

monster.description = "a deepling elite"
monster.experience = 3000
monster.outfit = {
	lookType = 441,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15176
monster.health = 3200
monster.maxHealth = 3200
monster.race = "blood"
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 86}, -- gold coin
	{id = 2149, chance = 6290, maxCount = 2}, -- small emerald
	{id = 2168, chance = 5360}, -- life ring
	{id = 5895, chance = 2000}, -- fish fin
	{id = 7590, chance = 24000}, -- great mana potion
	{id = 7591, chance = 25000}, -- great health potion
	{id = 13838, chance = 3380}, -- heavy trident
	{id = 13870, chance = 25000}, -- eye of a deepling
	{id = 15425, chance = 25000}, -- deepling warts
	{id = 15426, chance = 21700}, -- deeptags
	{id = 15451, chance = 640}, -- warrior's axe
	{id = 15452, chance = 19000}, -- deepling ridge
	{id = 15453, chance = 1234}, -- warrior's shield
	{id = 15488, chance = 25000}, -- deepling filet
	{id = 15649, chance = 24000, maxCount = 5}, -- vortex bolt
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -400, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -225, shootEffect = CONST_ANI_LARGEROCK, target = true, range = 7, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 10, minDamage = 150, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)
