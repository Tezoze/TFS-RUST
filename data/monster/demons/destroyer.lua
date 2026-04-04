local mType = Game.createMonsterType("Destroyer")
local monster = {}

monster.description = "a destroyer"
monster.experience = 2500
monster.outfit = {
	lookType = 236,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6320
monster.health = 3700
monster.maxHealth = 3700
monster.race = "undead"
monster.speed = 300
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
	canWalkOnFire = false,
	canWalkOnPoison = false,
	canWalkOnEnergy = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "COME HERE AND DIE!", yell = false},
	{text = "Destructiooooon!", yell = false},
	{text = "It's a good day to destroy!", yell = false},
}

monster.loot = {
	{id = 2125, chance = 578}, -- crystal necklace
	{id = 2148, chance = 60000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 60000, maxCount = 41}, -- gold coin
	{id = 2150, chance = 7692, maxCount = 2}, -- small amethyst
	{id = 2152, chance = 4166, maxCount = 3}, -- platinum coin
	{id = 2178, chance = 564}, -- mind stone
	{id = 2393, chance = 1694}, -- giant sword
	{id = 2416, chance = 14285}, -- crowbar
	{id = 2463, chance = 4347}, -- plate armor
	{id = 2489, chance = 10000}, -- dark armor
	{id = 2546, chance = 12500, maxCount = 12}, -- burst arrow
	{id = 2553, chance = 6250}, -- pick
	{id = 2645, chance = 992}, -- steel boots
	{id = 2666, chance = 50000, maxCount = 6}, -- meat
	{id = 5741, chance = 108}, -- skull helmet
	{id = 5944, chance = 6666}, -- soul orb
	{id = 6300, chance = 144}, -- death ring
	{id = 6500, chance = 20000}, -- demonic essence
	{id = 7419, chance = 833}, -- dreaded cleaver
	{id = 7427, chance = 869}, -- chaos mace
	{id = 7591, chance = 1136}, -- great health potion
	{id = 11215, chance = 7142}, -- metal spike
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -500, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -200, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 35,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 420, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -3},
	{type = COMBAT_ICEDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)