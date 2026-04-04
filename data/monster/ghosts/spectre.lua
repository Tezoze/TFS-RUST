local mType = Game.createMonsterType("Spectre")
local monster = {}

monster.description = "a spectre"
monster.experience = 2100
monster.outfit = {
	lookType = 235,
	lookHead = 20,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6348
monster.health = 1350
monster.maxHealth = 1350
monster.race = "undead"
monster.speed = 280
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Revenge ... is so ... sweet.", yell = false},
	{text = "Life...force! Feed me your... lifeforce", yell = false},
	{text = "Mor... tals!", yell = false},
	{text = "Buuuuuh", yell = false},
}

monster.loot = {
	{id = 2071, chance = 9620}, -- lyre
	{id = 2134, chance = 850}, -- silver brooch
	{id = 2134, chance = 110}, -- silver brooch
	{id = 2136, chance = 110}, -- demonbone amulet
	{id = 2148, chance = 33000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 33000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 33000, maxCount = 97}, -- gold coin
	{id = 2152, chance = 3850, maxCount = 7}, -- platinum coin
	{id = 2165, chance = 190}, -- stealth ring
	{id = 2189, chance = 9800}, -- wand of cosmic energy
	{id = 2260, chance = 30310, maxCount = 2}, -- blank rune
	{id = 5909, chance = 3800}, -- white piece of cloth
	{id = 5944, chance = 6005}, -- soul orb
	{id = 6300, chance = 280}, -- death ring
	{id = 6500, chance = 6270}, -- demonic essence
	{id = 7383, chance = 700}, -- relic sword
	{id = 7590, chance = 920}, -- great mana potion
	{id = 11227, chance = 1000}, -- shiny stone
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -308, target = false, condition = {type = CONDITION_POISON, startDamage = 300, interval = 2000}},
	{name = "drunk", interval = 2000, chance = 15, radius = 4, effect = CONST_ME_SOUND_PURPLE, target = false, duration = 6000},
	{name = "combat", interval = 2000, chance = 15, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -400, range = 7, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 20, minDamage = -300, maxDamage = -550, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 35,
	armor = 40,
	{name = "combat", interval = 2000, chance = 20, minDamage = 100, maxDamage = 700, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 290, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 90},
	{type = COMBAT_ICEDAMAGE, percent = 1},
	{type = COMBAT_FIREDAMAGE, percent = -8},
	{type = COMBAT_ENERGYDAMAGE, percent = -8},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)