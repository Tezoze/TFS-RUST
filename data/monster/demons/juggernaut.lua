local mType = Game.createMonsterType("Juggernaut")
local monster = {}

monster.description = "a juggernaut"
monster.experience = 4900
monster.outfit = {
	lookType = 244,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6336
monster.health = 20000
monster.maxHealth = 20000
monster.race = "blood"
monster.speed = 340
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 60,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "RAAARRR!", yell = false},
	{text = "GRRRRRR!", yell = false},
	{text = "WAHHHH!", yell = false},
}

monster.loot = {
	{id = 2136, chance = 550}, -- demonbone amulet
	{id = 2147, chance = 20000, maxCount = 4}, -- small ruby
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2149, chance = 20000, maxCount = 5}, -- small emerald
	{id = 2152, chance = 100000, maxCount = 15}, -- platinum coin
	{id = 2153, chance = 830}, -- violet gem
	{id = 2155, chance = 869}, -- green gem
	{id = 2156, chance = 13850}, -- red gem
	{id = 2434, chance = 9000}, -- dragon hammer
	{id = 2452, chance = 400}, -- heavy mace
	{id = 2454, chance = 400}, -- war axe
	{id = 2466, chance = 550}, -- golden armor
	{id = 2470, chance = 500}, -- golden legs
	{id = 2476, chance = 4990}, -- knight armor
	{id = 2514, chance = 800}, -- mastermind shield
	{id = 2578, chance = 280}, -- closed trap
	{id = 2671, chance = 60000, maxCount = 8}, -- ham
	{id = 5944, chance = 33333}, -- soul orb
	{id = 6500, chance = 45333}, -- demonic essence
	{id = 6558, chance = 25000, maxCount = 4}, -- concentrated demonic blood
	{id = 7365, chance = 11111, maxCount = 15}, -- onyx arrow
	{id = 7368, chance = 25000, maxCount = 10}, -- assassin star
	{id = 7413, chance = 4430}, -- titan axe
	{id = 7452, chance = 7761}, -- spiked squelcher
	{id = 7590, chance = 35000}, -- great mana potion
	{id = 7591, chance = 32000}, -- great health potion
	{id = 8889, chance = 400}, -- skullcracker armor
	{id = 9971, chance = 7692, maxCount = 2}, -- gold ingot
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -1470, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -780, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 60,
	armor = 70,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 520, duration = 5000},
	{name = "combat", interval = 2000, chance = 15, minDamage = 400, maxDamage = 900, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 40},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)