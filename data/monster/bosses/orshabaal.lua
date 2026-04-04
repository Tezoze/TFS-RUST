local mType = Game.createMonsterType("Orshabaal")
local monster = {}

monster.description = "Orshabaal"
monster.experience = 10000
monster.outfit = {
	lookType = 201,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5995
monster.health = 22500
monster.maxHealth = 22500
monster.race = "fire"
monster.speed = 380
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 2000,
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
	runHealth = 2500,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "PRAISED BE MY MASTERS, THE RUTHLESS SEVEN!", yell = false},
	{text = "YOU ARE DOOMED!", yell = false},
	{text = "ORSHABAAL IS BACK!", yell = false},
	{text = "Be prepared for the day my masters will come for you!", yell = false},
	{text = "SOULS FOR ORSHABAAL!", yell = false},
}

monster.loot = {
	{id = 1982, chance = 20000}, -- purple tome
	{id = 2033, chance = 12500}, -- golden mug
	{id = 2125, chance = 20000}, -- crystal necklace
	{id = 2143, chance = 33333, maxCount = 15}, -- white pearl
	{id = 2144, chance = 25000, maxCount = 8}, -- black pearl
	{id = 2145, chance = 20000, maxCount = 5}, -- small diamond
	{id = 2146, chance = 33333, maxCount = 8}, -- small sapphire
	{id = 2149, chance = 25000, maxCount = 7}, -- small emerald
	{id = 2150, chance = 20000, maxCount = 17}, -- small amethyst
	{id = 2151, chance = 20000, maxCount = 3}, -- talon
	{id = 2152, chance = 100000, maxCount = 69}, -- platinum coin
	{id = 2155, chance = 6666}, -- green gem
	{id = 2158, chance = 20000}, -- blue gem
	{id = 2162, chance = 6666}, -- magic light wand
	{id = 2164, chance = 6666}, -- might ring
	{id = 2170, chance = 20000}, -- silver amulet
	{id = 2171, chance = 12500}, -- platinum amulet
	{id = 2174, chance = 20000}, -- strange symbol
	{id = 2176, chance = 6666}, -- orb
	{id = 2177, chance = 12500}, -- life crystal
	{id = 2178, chance = 20000}, -- mind stone
	{id = 2195, chance = 12500}, -- boots of haste
	{id = 2200, chance = 20000}, -- protection amulet
	{id = 2214, chance = 33333}, -- ring of healing
	{id = 2377, chance = 12500}, -- two handed sword
	{id = 2393, chance = 25000}, -- giant sword
	{id = 2402, chance = 6666}, -- silver dagger
	{id = 2418, chance = 6666}, -- golden sickle
	{id = 2432, chance = 12500}, -- fire axe
	{id = 2434, chance = 6666}, -- dragon hammer
	{id = 2462, chance = 33333}, -- devil helmet
	{id = 2470, chance = 12500}, -- golden legs
	{id = 2472, chance = 6666}, -- magic plate armor
	{id = 2514, chance = 6666}, -- mastermind shield
	{id = 2520, chance = 25000}, -- demon shield
	{id = 5808, chance = 6666}, -- Orshabaal's brain
	{id = 2421, chance = 6666}, -- thunder hammer
	{id = 5954, chance = 50000}, -- demon horn
	{id = 6300, chance = 50000}, -- death ring
	{id = 6500, chance = 100000}, -- demonic essence
	{id = 7368, chance = 12500, maxCount = 42}, -- assassin star
	{id = 7590, chance = 33333}, -- great mana potion
	{id = 7591, chance = 20000}, -- great health potion
	{id = 8472, chance = 12500}, -- great spirit potion
	{id = 8473, chance = 33333}, -- ultimate health potion
	{id = 9971, chance = 6666}, -- gold ingot
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -1990, target = false},
	{name = "combat", interval = 1000, chance = 13, minDamage = -300, maxDamage = -600, range = 7, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 1000, chance = 6, minDamage = -150, maxDamage = -350, radius = 5, effect = CONST_ME_POISON, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 1000, chance = 6, radius = 5, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 34, minDamage = -310, maxDamage = -600, range = 7, radius = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 1000, chance = 10, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 1000, chance = 15, minDamage = -500, maxDamage = -850, effect = CONST_ME_ENERGY, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 111,
	armor = 90,
	{name = "combat", interval = 1000, chance = 9, minDamage = 1500, maxDamage = 2500, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 1000, chance = 17, minDamage = 600, maxDamage = 1000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 5, effect = CONST_ME_REDSHIMMER, speed = 1901, duration = 7000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = -1},
	{type = COMBAT_ICEDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "demon", chance = 10, interval = 1000, max = 4},
}

mType:register(monster)