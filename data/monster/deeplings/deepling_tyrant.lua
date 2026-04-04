local mType = Game.createMonsterType("Deepling Tyrant")
local monster = {}

monster.description = "a deepling tyrant"
monster.experience = 4500
monster.outfit = {
	lookType = 442,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15188
monster.health = 4200
monster.maxHealth = 4200
monster.race = "blood"
monster.speed = 260
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
	runHealth = 20,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "QJELL NETA NA!!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 97}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 97}, -- gold coin
	{id = 2152, chance = 70000, maxCount = 4}, -- platinum coin
	{id = 7590, chance = 32640, maxCount = 3}, -- great mana potion
	{id = 7591, chance = 33430, maxCount = 3}, -- great health potion
	{id = 13870, chance = 28850}, -- eye of a deepling
	{id = 15423, chance = 23100}, -- deepling guard belt buckle
	{id = 15424, chance = 34770}, -- deepling breaktime snack
	{id = 15454, chance = 1420}, -- guardian axe
	{id = 15455, chance = 29960}, -- deepling claw
	{id = 15545, chance = 80}, -- foxtail
	{id = 15645, chance = 510}, -- deepling backpack
	{id = 15647, chance = 1540}, -- deepling squelcher
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -501, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -375, shootEffect = CONST_ANI_WHIRLWINDSWORD, target = true, range = 7, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -180, maxDamage = -215, shootEffect = CONST_ANI_SPEAR, effect = CONST_ME_BLUE_BUBBLE, target = true, range = 7, type = COMBAT_DROWNDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 45,
	{name = "combat", interval = 2000, chance = 15, minDamage = 200, maxDamage = 400, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)
