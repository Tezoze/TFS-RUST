local mType = Game.createMonsterType("Merlkin")
local monster = {}

monster.description = "a merlkin"
monster.experience = 145
monster.outfit = {
	lookType = 117,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6044
monster.health = 235
monster.maxHealth = 235
monster.race = "blood"
monster.speed = 194
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
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ugh! Ugh! Ugh!", yell = false},
	{text = "Holy banana!", yell = false},
	{text = "Chakka! Chakka!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 69500, maxCount = 45}, -- gold coin
	{id = 2150, chance = 260}, -- small amethyst
	{id = 2162, chance = 3000}, -- magic light wand
	{id = 2188, chance = 1050}, -- wand of decay
	{id = 2675, chance = 1000, maxCount = 5}, -- orange
	{id = 2676, chance = 30350, maxCount = 12}, -- banana
	{id = 3966, chance = 5000}, -- banana staff
	{id = 5883, chance = 3000}, -- ape fur
	{id = 7620, chance = 660}, -- mana potion
	{id = 12467, chance = 1800}, -- banana sash
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -90, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -15, maxDamage = -45, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "poisonfield", interval = 2000, chance = 15, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 16,
	{name = "combat", interval = 2000, chance = 25, minDamage = 30, maxDamage = 40, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)