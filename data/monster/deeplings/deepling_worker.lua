local mType = Game.createMonsterType("Deepling Worker")
local monster = {}

monster.description = "a deepling worker"
monster.experience = 130
monster.outfit = {
	lookType = 470,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15497
monster.health = 190
monster.maxHealth = 190
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 20,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Qjell afar gou jey!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 70000, maxCount = 25}, -- gold coin
	{id = 2149, chance = 110, maxCount = 3}, -- small emerald
	{id = 2667, chance = 12020, maxCount = 3}, -- fish
	{id = 5895, chance = 350}, -- fish fin
	{id = 13838, chance = 510}, -- heavy trident
	{id = 13870, chance = 283}, -- eye of a deepling
	{id = 15430, chance = 6950}, -- deepling scales
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -50, shootEffect = CONST_ANI_SPEAR, target = true, range = 7, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -20},
	{type = COMBAT_EARTHDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)
