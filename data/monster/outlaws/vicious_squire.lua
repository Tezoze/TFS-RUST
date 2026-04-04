local mType = Game.createMonsterType("Vicious Squire")
local monster = {}

monster.description = "a vicious squire"
monster.experience = 900
monster.outfit = {
	lookType = 131,
	lookHead = 97,
	lookBody = 26,
	lookLegs = 71,
	lookFeet = 114,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 24673
monster.health = 1000
monster.maxHealth = 1000
monster.race = "blood"
monster.speed = 240
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
	{text = "Your stuff will be mine soon!", yell = false},
	{text = "I'll cut you a bloody grin!", yell = false},
	{text = "For hurting me, my sire will kill you!", yell = false},
	{text = "You shouldn't have come here!", yell = false},
}

monster.loot = {
	{id = 2543, chance = 90450, maxCount = 10}, -- bolt
	{id = 2148, chance = 75410, maxCount = 30}, -- gold coin
	{id = 2681, chance = 15400}, -- grapes
	{id = 7591, chance = 12340, maxCount = 2}, -- great health potion
	{id = 2666, chance = 5000}, -- meat
	{id = 2455, chance = 830}, -- crossbow
	{id = 2652, chance = 760}, -- green tunic
	{id = 2164, chance = 700, maxCount = 2}, -- might ring
	{id = 2120, chance = 1000}, -- rope
	{id = 2661, chance = 1000}, -- scarf
	{id = 1949, chance = 830}, -- scroll
	{id = 2145, chance = 830}, -- small diamond
	{id = 2391, chance = 130}, -- war hammer
	{id = 2381, chance = 830}, -- halberd
	{id = 2515, chance = 330}, -- guardian shield
	{id = 2477, chance = 230}, -- knight legs
	{id = 2475, chance = 200}, -- warrior helmet
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 10, maxDamage = -175, target = false},
	{name = "combat", interval = 2000, chance = 40, minDamage = 10, maxDamage = -100, range = 7, shootEffect = CONST_ANI_BOLT, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 50,
	armor = 35,
	{name = "combat", interval = 4000, chance = 25, minDamage = 20, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

mType:register(monster)
