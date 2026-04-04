local mType = Game.createMonsterType("Renegade Knight")
local monster = {}

monster.description = "a renegade knight"
monster.experience = 1200
monster.outfit = {
	lookType = 268,
	lookHead = 97,
	lookBody = 132,
	lookLegs = 76,
	lookFeet = 98,
	lookAddons = 2,
	lookMount = 0
}

monster.corpse = 24676
monster.health = 1450
monster.maxHealth = 1450
monster.race = "blood"
monster.speed = 280
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 20
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
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

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "I'll teach you a lesson!", yell = false},
	{text = "Feel my steel!", yell = false},
	{text = "Take this!", yell = false},
	{text = "Let's see how good you are!", yell = false},
	{text = "A challenge at last!", yell = false},
}

monster.loot = {
	{id = 2544, chance = 90450, maxCount = 10}, -- arrow
	{id = 2148, chance = 75410, maxCount = 30}, -- gold coin
	{id = 2681, chance = 1210}, -- grapes
	{id = 7591, chance = 1210}, -- great health potion
	{id = 2666, chance = 1210, maxCount = 2}, -- meat
	{id = 7364, chance = 1210, maxCount = 4}, -- sniper arrow
	{id = 2487, chance = 210}, -- crown armor
	{id = 2491, chance = 310}, -- crown helmet
	{id = 2519, chance = 210}, -- crown shield
	{id = 2488, chance = 110}, -- crown legs
	{id = 2392, chance = 310}, -- fire sword
	{id = 2381, chance = 1610}, -- halberd
	{id = 2744, chance = 510}, -- red rose
	{id = 2120, chance = 1510}, -- rope
	{id = 1949, chance = 910}, -- scroll
	{id = 12466, chance = 910}, -- scroll of heroic deeds
	{id = 12406, chance = 910}, -- small notebook
	{id = 2121, chance = 510}, -- wedding ring
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 10, maxDamage = -175, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = -25, maxDamage = -75, radius = 3, effect = CONST_ME_BLOCKHIT, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 50,
	armor = 35,
	{name = "combat", interval = 4000, chance = 25, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

mType:register(monster)
