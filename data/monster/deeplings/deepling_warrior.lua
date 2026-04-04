local mType = Game.createMonsterType("Deepling Warrior")
local monster = {}

monster.description = "a deepling warrior"
monster.experience = 1500
monster.outfit = {
	lookType = 441,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15175
monster.health = 1600
monster.maxHealth = 1600
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
	staticAttackChance = 70,
	runHealth = 30,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Jou wjil all djie!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 60000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 60000, maxCount = 80}, -- gold coin
	{id = 2149, chance = 2854}, -- small emerald
	{id = 2168, chance = 2941}, -- life ring
	{id = 5895, chance = 961}, -- fish fin
	{id = 7590, chance = 9090}, -- great mana potion
	{id = 7591, chance = 11111}, -- great health potion
	{id = 13838, chance = 1030}, -- heavy trident
	{id = 13870, chance = 574}, -- eye of a deepling
	{id = 15425, chance = 10000}, -- deepling warts
	{id = 15426, chance = 14285}, -- deeptags
	{id = 15451, chance = 534}, -- warrior's axe
	{id = 15452, chance = 11111}, -- deepling ridge
	{id = 15453, chance = 684}, -- warrior's shield
	{id = 15488, chance = 14285}, -- deepling filet
	{id = 15649, chance = 3571, maxCount = 5}, -- vortex bolt
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -290, shootEffect = CONST_ANI_WHIRLWINDAXE, target = true, range = 7, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 2000, chance = 15, minDamage = 50, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -20},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)
