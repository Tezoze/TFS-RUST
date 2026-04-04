local mType = Game.createMonsterType("Draken Spellweaver")
local monster = {}

monster.description = "a draken spellweaver"
monster.experience = 3100
monster.outfit = {
	lookType = 340,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11316
monster.health = 5000
monster.maxHealth = 5000
monster.race = "blood"
monster.speed = 336
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
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Kazzzzzzuuum!", yell = false},
	{text = "Fissziss!", yell = false},
	{text = "Zzzzzooom!", yell = false},
}

monster.loot = {
	{id = 2123, chance = 370}, -- ring of the sky
	{id = 2147, chance = 6910, maxCount = 5}, -- small ruby
	{id = 2148, chance = 41000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 58000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 25510, maxCount = 5}, -- platinum coin
	{id = 2155, chance = 970}, -- green gem
	{id = 2187, chance = 1660}, -- wand of inferno
	{id = 2666, chance = 30400}, -- meat
	{id = 7590, chance = 4970}, -- great mana potion
	{id = 8871, chance = 1450}, -- focus cape
	{id = 11303, chance = 1980}, -- Zaoan shoes
	{id = 11314, chance = 19790}, -- weaver's wandtip
	{id = 11315, chance = 10}, -- draken trophy
	{id = 11355, chance = 620}, -- spellweaver's robe
	{id = 11356, chance = 770}, -- Zaoan robe
	{id = 12410, chance = 1980}, -- luminous orb
	{id = 12614, chance = 3930}, -- draken sulphur
	{id = 11134, chance = 1000}, -- Tome of Knowledge
	{id = 13294, chance = 200}, -- harness
	{id = 13538, chance = 220}, -- bamboo leaves
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -252, interval = 2000, target = false},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -240, maxDamage = -480, interval = 2000, chance = 10, length = 4, spread = 3, target = false, effect = CONST_ME_EXPLOSION},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -100, maxDamage = -250, interval = 2000, chance = 10, range = 7, target = true, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA},
	{name = "combat", type = COMBAT_ENERGYDAMAGE, minDamage = -150, maxDamage = -300, interval = 2000, chance = 10, range = 7, target = true, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -200, maxDamage = -380, interval = 2000, chance = 10, radius = 4, target = true, effect = CONST_ME_POFF},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -280, maxDamage = -360, range = 7, shootEffect = CONST_ANI_POISON, target = true},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_REDSHIMMER},
	{name = "combat", interval = 2000, chance = 15, minDamage = 270, maxDamage = 530, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 75},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)