local mType = Game.createMonsterType("Zombie")
local monster = {}

monster.description = "a zombie"
monster.experience = 280
monster.outfit = {
	lookType = 311,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9875
monster.health = 500
monster.maxHealth = 500
monster.race = "undead"
monster.speed = 180
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
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Mst.... klll....", yell = false},
	{text = "Whrrrr... ssss.... mmm.... grrrrl", yell = false},
	{text = "Dnnnt... cmmm... clsrrr....", yell = false},
	{text = "Httt.... hmnnsss...", yell = false},
}

monster.loot = {
	{id = 2148, chance = 82000, maxCount = 65}, -- gold coin
	{id = 2381, chance = 3750}, -- halberd
	{id = 2398, chance = 7250}, -- mace
	{id = 2417, chance = 7000}, -- battle hammer
	{id = 2457, chance = 4600}, -- steel helmet
	{id = 2460, chance = 9400}, -- brass helmet
	{id = 2657, chance = 560}, -- simple dress
	{id = 7620, chance = 740}, -- mana potion
	{id = 9808, chance = 5680},
	{id = 10576, chance = 10000}, -- half-eaten brain
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -130, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -14, maxDamage = -23, range = 1, target = true, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = -15, maxDamage = -24, range = 7, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -49, range = 1, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 50},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)