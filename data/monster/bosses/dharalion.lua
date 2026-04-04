local mType = Game.createMonsterType("Dharalion")
local monster = {}

monster.description = "Dharalion"
monster.experience = 570
monster.outfit = {
	lookType = 203,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6011
monster.health = 380
monster.maxHealth = 380
monster.race = "blood"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Feel my wrath!", yell = false},
	{text = "No one will stop my ascension!", yell = false},
	{text = "You desecrated this temple!", yell = false},
	{text = "Muahahaha!", yell = false},
	{text = "My powers are divine!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 20}, -- gold coin
	{id = 2260, chance = 4000}, -- blank rune
	{id = 2682, chance = 6666}, -- melon
	{id = 2802, chance = 10000}, -- sling herb
	{id = 2177, chance = 2857}, -- life crystal
	{id = 2689, chance = 20000, maxCount = 3}, -- bread
	{id = 2652, chance = 5000}, -- green tunic
	{id = 2032, chance = 4000}, -- bowl
	{id = 2154, chance = 1333}, -- yellow gem
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 30, attack = 28, target = false},
	{name = "combat", interval = 1000, chance = 15, minDamage = -30, maxDamage = -60, range = 7, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 1000, chance = 13, minDamage = -70, maxDamage = -90, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 10, minDamage = -80, maxDamage = -151, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 13, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 15,
	{name = "combat", interval = 1000, chance = 20, minDamage = 90, maxDamage = 120, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 7, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 10000},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "poison", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "demon skeleton", chance = 6, interval = 1000, max = 2},
}

mType:register(monster)