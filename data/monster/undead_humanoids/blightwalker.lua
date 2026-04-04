local mType = Game.createMonsterType("Blightwalker")
local monster = {}

monster.description = "a blightwalker"
monster.experience = 5850
monster.outfit = {
	lookType = 246,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6354
monster.health = 8900
monster.maxHealth = 8900
monster.race = "undead"
monster.speed = 350
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
	runHealth = 800,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "I can see you decaying!", yell = false},
	{text = "Let me taste your mortality!", yell = false},
	{text = "Your lifeforce is waning!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 197}, -- gold coin
	{id = 2152, chance = 100000, maxCount = 5}, -- platinum coin
	{id = 2173, chance = 120}, -- amulet of loss
	{id = 2179, chance = 1870}, -- gold ring
	{id = 2183, chance = 10000}, -- hailstorm rod
	{id = 2199, chance = 2050}, -- garlic necklace
	{id = 2260, chance = 26250, maxCount = 2}, -- blank rune
	{id = 2418, chance = 350}, -- golden sickle
	{id = 2436, chance = 1520}, -- skull staff
	{id = 2550, chance = 3000}, -- scythe
	{id = 2694, chance = 50000}, -- bunch of wheat
	{id = 5944, chance = 23720}, -- soul orb
	{id = 6300, chance = 1410}, -- death ring
	{id = 6500, chance = 28000}, -- demonic essence
	{id = 7368, chance = 5900, maxCount = 10}, -- assassin star
	{id = 7590, chance = 31360, maxCount = 3}, -- great mana potion
	{id = 7632, chance = 4450},
	{id = 7633, chance = 4450},
	{id = 7732, chance = 4300}, -- seeds
	{id = 7884, chance = 1050}, -- terra mantle
	{id = 7885, chance = 2500}, -- terra legs
	{id = 8473, chance = 14720, maxCount = 2}, -- ultimate health potion
	{id = 9971, chance = 5270}, -- gold ingot
	{id = 10605, chance = 15000}, -- bundle of cursed straw
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -490, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -220, maxDamage = -405, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -65, maxDamage = -135, radius = 4, effect = CONST_ME_GREENSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, radius = 3, effect = CONST_ME_GREENSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 2000, chance = 15, range = 7, shootEffect = CONST_ANI_POISON, target = true, speed = -300, duration = 30000},
}

monster.defenses = {
	defense = 50,
	armor = 63,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -30},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)