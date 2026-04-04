local mType = Game.createMonsterType("Elf Arcanist")
local monster = {}

monster.description = "an elf arcanist"
monster.experience = 175
monster.outfit = {
	lookType = 63,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6011
monster.health = 220
monster.maxHealth = 220
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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Feel my wrath!", yell = false},
	{text = "For the Daughter of the Stars!", yell = false},
	{text = "I'll bring balance upon you!", yell = false},
	{text = "Tha'shi Cenath", yell = false},
	{text = "Vihil Ealuel", yell = false},
}

monster.loot = {
	{id = 1949, chance = 31000}, -- scroll
	{id = 2047, chance = 2100}, -- candlestick
	{id = 2148, chance = 37000, maxCount = 47}, -- gold coin
	{id = 2154, chance = 50}, -- yellow gem
	{id = 2177, chance = 970}, -- life crystal
	{id = 2189, chance = 1160}, -- wand of cosmic energy
	{id = 2198, chance = 1999}, -- elven amulet
	{id = 2260, chance = 18000}, -- blank rune
	{id = 2544, chance = 6000, maxCount = 3}, -- arrow
	{id = 2600, chance = 1000}, -- inkwell
	{id = 2642, chance = 950}, -- sandals
	{id = 2652, chance = 7000}, -- green tunic
	{id = 2682, chance = 22000}, -- melon
	{id = 2689, chance = 14000}, -- bread
	{id = 2747, chance = 880}, -- grave flower
	{id = 2802, chance = 5000}, -- sling herb
	{id = 5922, chance = 5100}, -- holy orchid
	{id = 7589, chance = 3000}, -- strong mana potion
	{id = 7618, chance = 4000}, -- health potion
	{id = 10552, chance = 10000}, -- elvish talisman
	{id = 12421, chance = 7710}, -- elven astral observer
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -70, range = 7, shootEffect = CONST_ANI_ARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -30, maxDamage = -50, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -70, maxDamage = -85, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 15, minDamage = 40, maxDamage = 60, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)