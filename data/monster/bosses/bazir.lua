local mType = Game.createMonsterType("Bazir")
local monster = {}

monster.description = "Bazir"
monster.experience = 30000
monster.outfit = {
	lookType = 35,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 2916
monster.health = 110000
monster.maxHealth = 110000
monster.race = "fire"
monster.speed = 530
monster.manaCost = 0
monster.maxSummons = 1

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
	runHealth = 3000,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "COME HERE! FREE ITEMS FOR EVERYONE!", yell = true},
	{text = "BOW TO THE POWER OF THE RUTHLESS SEVEN!", yell = false},
	{text = "Slay your friends and I will spare you!", yell = true},
	{text = "DON'T BE AFRAID! I AM COMING IN PEACE!", yell = true},
}

monster.loot = {
	{id = 2142, chance = 3500}, -- ancient amulet
	{id = 2231, chance = 9000}, -- big bone
	{id = 2144, chance = 15000, maxCount = 15}, -- black pearl
	{id = 2158, chance = 1500}, -- blue gem
	{id = 2195, chance = 4000}, -- boots of haste
	{id = 2186, chance = 3500}, -- moonlight rod
	{id = 2192, chance = 2500}, -- crystal ball
	{id = 2125, chance = 1500}, -- crystal necklace
	{id = 2124, chance = 5500}, -- crystal ring
	{id = 2520, chance = 15500}, -- demon shield
	{id = 2462, chance = 11000}, -- devil helmet
	{id = 2387, chance = 20000}, -- double axe
	{id = 2434, chance = 4500}, -- dragon hammer
	{id = 2167, chance = 13500}, -- energy ring
	{id = 2432, chance = 17000}, -- fire axe
	{id = 2393, chance = 12500}, -- giant sword
	{id = 2148, chance = 99900, maxCount = 100}, -- gold coin
	{id = 2148, chance = 88800, maxCount = 100}, -- gold coin
	{id = 2148, chance = 77700, maxCount = 100}, -- gold coin
	{id = 2148, chance = 66600, maxCount = 100}, -- gold coin
	{id = 2179, chance = 8000}, -- gold ring
	{id = 2470, chance = 5000}, -- golden legs
	{id = 2033, chance = 7500}, -- golden mug
	{id = 2418, chance = 4500}, -- golden sickle
	{id = 2182, chance = 3500}, -- snakebite rod
	{id = 2155, chance = 1500}, -- green gem
	{id = 2188, chance = 2500}, -- wand of decay
	{id = 2396, chance = 7500}, -- ice rapier
	{id = 2177, chance = 1000}, -- life crystal
	{id = 2162, chance = 11500}, -- magic light wand
	{id = 2472, chance = 3000}, -- magic plate armor
	{id = 2514, chance = 7500}, -- mastermind shield
	{id = 2164, chance = 5000}, -- might ring
	{id = 2178, chance = 4000}, -- mind stone
	{id = 2176, chance = 12000}, -- orb
	{id = 2171, chance = 4500}, -- platinum amulet
	{id = 2200, chance = 4500}, -- protection amulet
	{id = 1982, chance = 2600}, -- purple tome
	{id = 2214, chance = 13000}, -- ring of healing
	{id = 2123, chance = 3500}, -- ring of the sky
	{id = 2170, chance = 13000}, -- silver amulet
	{id = 2402, chance = 15500}, -- silver dagger
	{id = 2436, chance = 5000}, -- skull staff
	{id = 2150, chance = 13500, maxCount = 20}, -- small amethyst
	{id = 2145, chance = 9500, maxCount = 5}, -- small diamond
	{id = 2149, chance = 15500, maxCount = 10}, -- small emerald
	{id = 2146, chance = 13500, maxCount = 10}, -- small sapphire
	{id = 2165, chance = 9500}, -- stealth ring
	{id = 2197, chance = 4000}, -- stone skin amulet
	{id = 2174, chance = 2500}, -- strange symbol
	{id = 2151, chance = 14000, maxCount = 7}, -- talon
	{id = 2112, chance = 14500}, -- teddy bear
	{id = 2421, chance = 13500}, -- thunder hammer
	{id = 2377, chance = 20000}, -- two handed sword
	{id = 3955, chance = 100}, -- voodoo doll
	{id = 2185, chance = 3500}, -- necrotic rod
	{id = 2143, chance = 12500, maxCount = 15}, -- white pearl
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 200, attack = 250, target = false},
	{name = "combat", interval = 1000, chance = 7, minDamage = -100, maxDamage = -1000, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_POFF, target = true, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 1000, chance = 7, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGYAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 9, range = 7, shootEffect = CONST_ANI_LARGEROCK, effect = CONST_ME_ENERGYAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 13, minDamage = -400, maxDamage = -700, radius = 8, effect = CONST_ME_BLUEBUBBLE, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 1000, chance = 10, minDamage = -400, maxDamage = -700, radius = 8, effect = CONST_ME_GREENSHIMMER, target = false, type = COMBAT_MANADRAIN},
	{name = "speed", interval = 1000, chance = 12, radius = 6, effect = CONST_ME_POISON, target = false, speed = -1000, duration = 60000},
	{name = "combat", interval = 1000, chance = 8, radius = 5, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "outfit", interval = 1000, chance = 2, radius = 8, effect = CONST_ME_BLUEBUBBLE, target = false},
	{name = "outfit", interval = 1000, chance = 2, radius = 8, effect = CONST_ME_BLUEBUBBLE, target = false},
	{name = "combat", interval = 1000, chance = 34, minDamage = -100, maxDamage = -900, range = 7, radius = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 15, minDamage = -500, maxDamage = -850, effect = CONST_ME_REDSHIMMER, target = false, length = 8, spread = 0, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 160,
	armor = 160,
	{name = "combat", interval = 1000, chance = 15, minDamage = 5000, maxDamage = 10000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 1000, chance = 25, minDamage = 2000, maxDamage = 3000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 8, effect = CONST_ME_REDSHIMMER, speed = 1901, duration = 5000},
	{name = "invisible", interval = 1000, chance = 4, effect = CONST_ME_BLUESHIMMER},
	{name = "invisible", interval = 1000, chance = 17, effect = CONST_ME_BLUESHIMMER},
	{name = "outfit", interval = 1000, chance = 2, effect = CONST_ME_BLUESHIMMER, item = 2916, duration = 7000},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "poison", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "bazir", chance = 15, interval = 1000, max = 1},
}

mType:register(monster)