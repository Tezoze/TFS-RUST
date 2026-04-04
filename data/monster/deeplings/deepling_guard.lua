local mType = Game.createMonsterType("Deepling Guard")
local monster = {}

monster.description = "a deepling guard"
monster.experience = 2100
monster.outfit = {
	lookType = 442,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15187
monster.health = 1900
monster.maxHealth = 1900
monster.race = "blood"
monster.speed = 220
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
	canPushCreatures = false,
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
	{id = 2146, chance = 2890, maxCount = 3}, -- small sapphire
	{id = 2148, chance = 100000, maxCount = 90}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 90}, -- gold coin
	{id = 2152, chance = 70000, maxCount = 2}, -- platinum coin
	{id = 7590, chance = 14285, maxCount = 3}, -- great mana potion
	{id = 7591, chance = 14285, maxCount = 3}, -- great health potion
	{id = 13838, chance = 1694}, -- heavy trident
	{id = 13870, chance = 10000}, -- eye of a deepling
	{id = 15423, chance = 12500}, -- deepling guard belt buckle
	{id = 15424, chance = 16666}, -- deepling breaktime snack
	{id = 15454, chance = 925}, -- guardian axe
	{id = 15455, chance = 9090}, -- deepling claw
	{id = 15545, chance = 10}, -- foxtail
	{id = 15644, chance = 362}, -- ornate crossbow
	{id = 15645, chance = 333}, -- deepling backpack
	{id = 15647, chance = 751}, -- deepling squelcher
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -400, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -200, shootEffect = CONST_ANI_WHIRLWINDSWORD, target = true, range = 7, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -150, shootEffect = CONST_ANI_SPEAR, effect = CONST_ME_BLUE_BUBBLE, target = true, range = 7, type = COMBAT_DROWNDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 35,
	{name = "combat", interval = 2000, chance = 10, minDamage = 100, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -20},
	{type = COMBAT_EARTHDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)
