local mType = Game.createMonsterType("Vile Grandmaster")
local monster = {}

monster.description = "a vile grandmaster"
monster.experience = 1500
monster.outfit = {
	lookType = 268,
	lookHead = 97,
	lookBody = 0,
	lookLegs = 95,
	lookFeet = 94,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 24679
monster.health = 1700
monster.maxHealth = 1700
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
	{text = "Is that the best, you can throw at me?", yell = false},
	{text = "I've seen orcs tougher than you!", yell = false},
	{text = "I will end this now!", yell = false},
	{text = "Your gods won't help you!", yell = false},
	{text = "You'll make a fine trophy!", yell = false},
}

monster.loot = {
	{id = 7364, chance = 1210, maxCount = 4}, -- sniper arrow
	{id = 2148, chance = 75410, maxCount = 30}, -- gold coin
	{id = 2152, chance = 75410, maxCount = 2}, -- platinum coin
	{id = 2681, chance = 1210}, -- grapes
	{id = 2666, chance = 1210, maxCount = 2}, -- meat
	{id = 7591, chance = 1210}, -- great health potion
	{id = 2381, chance = 1610}, -- halberd
	{id = 2744, chance = 510}, -- red rose
	{id = 2120, chance = 1510}, -- rope
	{id = 12466, chance = 910}, -- scroll of heroic deeds
	{id = 12406, chance = 910}, -- small notebook
	{id = 2147, chance = 810, maxCount = 2}, -- small ruby
	{id = 2146, chance = 810, maxCount = 2}, -- small sapphire
	{id = 2121, chance = 510}, -- wedding ring
	{id = 5911, chance = 210}, -- red piece of cloth
	{id = 2391, chance = 210}, -- war hammer
	{id = 2487, chance = 310}, -- crown armor
	{id = 2392, chance = 210}, -- fire sword
	{id = 2491, chance = 310}, -- crown helmet
	{id = 2519, chance = 210}, -- crown shield
	{id = 2488, chance = 110}, -- crown legs
	{id = 2171, chance = 210}, -- platinum amulet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 10, maxDamage = -260, target = false},
	{name = "condition", type = CONDITION_BLEEDING, interval = 2000, chance = 20, minDamage = -150, maxDamage = -225, radius = 4, shootEffect = CONST_ANI_THROWINGKNIFE, effect = CONST_ME_DRAWBLOOD, target = true},
	{name = "combat", interval = 2000, chance = 15, minDamage = -35, maxDamage = -105, radius = 5, effect = CONST_ME_GROUNDSHAKER, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 50,
	armor = 35,
	{name = "combat", interval = 4000, chance = 15, minDamage = 220, maxDamage = 280, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
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
