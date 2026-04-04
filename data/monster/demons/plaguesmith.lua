local mType = Game.createMonsterType("Plaguesmith")
local monster = {}

monster.description = "a plaguesmith"
monster.experience = 4500
monster.outfit = {
	lookType = 247,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6516
monster.health = 8250
monster.maxHealth = 8250
monster.race = "venom"
monster.speed = 320
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
	staticAttackChance = 70,
	runHealth = 500,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You are looking a bit feverish!", yell = false},
	{text = "You don't look that good!", yell = false},
	{text = "Hachoo!", yell = false},
	{text = "Cough Cough", yell = false},
}

monster.loot = {
	{id = 2127, chance = 341}, -- emerald bangle
	{id = 2134, chance = 2000}, -- silver brooch
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 65}, -- gold coin
	{id = 2150, chance = 5000, maxCount = 3}, -- small amethyst
	{id = 2152, chance = 7142, maxCount = 2}, -- platinum coin
	{id = 2207, chance = 4347}, -- melee ring
	{id = 2207, chance = 4761}, -- melee ring
	{id = 2225, chance = 20000}, -- piece of iron
	{id = 2235, chance = 50000}, -- mouldy cheese
	{id = 2237, chance = 60000}, -- dirty cape
	{id = 2377, chance = 20000}, -- two handed sword
	{id = 2391, chance = 2127}, -- war hammer
	{id = 2394, chance = 29000}, -- morning star
	{id = 2417, chance = 20000}, -- battle hammer
	{id = 2444, chance = 952}, -- hammer of wrath
	{id = 2477, chance = 6250}, -- knight legs
	{id = 2509, chance = 20000}, -- steel shield
	{id = 2645, chance = 1123}, -- steel boots
	{id = 5887, chance = 1234}, -- piece of royal steel
	{id = 5888, chance = 1010}, -- piece of hell steel
	{id = 5889, chance = 1030}, -- piece of draconian steel
	{id = 5944, chance = 11111}, -- soul orb
	{id = 6500, chance = 9033}, -- demonic essence
	{id = 7365, chance = 7692, maxCount = 4}, -- onyx arrow
	{id = 7591, chance = 10000}, -- great health potion
	{id = 9810, chance = 540},
}

monster.attacks = {
	{name = "melee", interval = 1500, minDamage = 0, maxDamage = -539, target = false, condition = {type = CONDITION_POISON, startDamage = 200, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -114, radius = 4, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -350, effect = CONST_ME_YELLOWBUBBLE, target = false, length = 5, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 2000, chance = 15, radius = 4, effect = CONST_ME_POISON, target = false, speed = -800, duration = 30000},
}

monster.defenses = {
	defense = 40,
	armor = 30,
	{name = "combat", interval = 2000, chance = 10, minDamage = 200, maxDamage = 280, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 440, duration = 5000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 1},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)