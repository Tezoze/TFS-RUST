local mType = Game.createMonsterType("Morgaroth")
local monster = {}

monster.description = "Morgaroth"
monster.experience = 15000
monster.outfit = {
	lookType = 12,
	lookHead = 0,
	lookBody = 94,
	lookLegs = 79,
	lookFeet = 79,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6068
monster.health = 55000
monster.maxHealth = 55000
monster.race = "fire"
monster.speed = 400
monster.manaCost = 0
monster.maxSummons = 6

monster.changeTarget = {
	interval = 10000,
	chance = 20
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
	staticAttackChance = 98,
	targetDistance = 1,
	runHealth = 100,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 30,
	{text = "I AM MORGAROTH, LORD OF THE TRIANGLE... AND YOU ARE LOST!", yell = true},
	{text = "MY SEED IS FEAR AND MY HARVEST ARE YOUR SOULS!", yell = true},
	{text = "ZATHROTH! LOOK AT THE DESTRUCTION I AM CAUSING IN YOUR NAME!", yell = true},
	{text = "THE TRIANGLE OF TERROR WILL RISE!", yell = true},
}

monster.loot = {
	{id = 2152, chance = 95000, maxCount = 74}, -- platinum coin
	{id = 6500, chance = 95000, maxCount = 5}, -- demonic essence
	{id = 2155, chance = 50000}, -- green gem
	{id = 7590, chance = 45000}, -- great mana potion
	{id = 2150, chance = 36000, maxCount = 18}, -- small amethyst
	{id = 8852, chance = 36000}, -- The devileye
	{id = 2149, chance = 27000, maxCount = 7}, -- small emerald
	{id = 2146, chance = 27000, maxCount = 9}, -- small sapphire
	{id = 1986, chance = 27000}, -- red tome
	{id = 8473, chance = 27000}, -- ultimate health potion
	{id = 2151, chance = 22000, maxCount = 7}, -- talon
	{id = 5954, chance = 22000, maxCount = 2}, -- demon horn
	{id = 6300, chance = 22000}, -- death ring
	{id = 2214, chance = 22000}, -- ring of healing
	{id = 8850, chance = 22000}, -- chain bolter
	{id = 8865, chance = 22000}, -- dark lord's cape
	{id = 8853, chance = 25000}, -- The ironworker
	{id = 2387, chance = 18000}, -- double axe
	{id = 8472, chance = 18000}, -- great spirit potion
	{id = 2472, chance = 18000}, -- magic plate armor
	{id = 2164, chance = 18000}, -- might ring
	{id = 2178, chance = 18000}, -- mind stone
	{id = 2165, chance = 18000}, -- stealth ring
	{id = 8881, chance = 18000}, -- fireborn giant armor
	{id = 8851, chance = 18000}, -- royal crossbow
	{id = 2112, chance = 18000}, -- teddy bear
	{id = 2143, chance = 13000, maxCount = 11}, -- white pearl
	{id = 2144, chance = 13000, maxCount = 13}, -- black pearl
	{id = 7368, chance = 13000, maxCount = 35}, -- assassin star
	{id = 7431, chance = 13000}, -- demonbone
	{id = 2033, chance = 13000}, -- golden mug
	{id = 5943, chance = 13000}, -- Morgaroth's heart
	{id = 8928, chance = 13000}, -- obsidian truncheon
	{id = 8929, chance = 13000}, -- The stomper
	{id = 2158, chance = 9000}, -- blue gem
	{id = 2179, chance = 9000}, -- gold ring
	{id = 2520, chance = 9000}, -- demon shield
	{id = 2167, chance = 9000}, -- energy ring
	{id = 2393, chance = 9000}, -- giant sword
	{id = 2470, chance = 9000}, -- golden legs
	{id = 2177, chance = 9000}, -- life crystal
	{id = 2162, chance = 9000}, -- magic light wand
	{id = 2176, chance = 9000}, -- orb
	{id = 2174, chance = 9000}, -- strange symbol
	{id = 2645, chance = 9000}, -- steel boots
	{id = 2421, chance = 9000}, -- thunder hammer
	{id = 2145, chance = 4500, maxCount = 5}, -- small diamond
	{id = 2124, chance = 4500}, -- crystal ring
	{id = 2432, chance = 4500}, -- fire axe
	{id = 7591, chance = 4500}, -- great health potion
	{id = 2514, chance = 4500}, -- mastermind shield
	{id = 8867, chance = 4500}, -- dragon robe
	{id = 8886, chance = 4500}, -- molten plate
	{id = 2522, chance = 500}, -- great shield
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -2250, target = false},
	{name = "combat", interval = 3000, chance = 35, minDamage = -500, maxDamage = -1210, range = 7, radius = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1800, chance = 40, minDamage = 0, maxDamage = -580, range = 7, radius = 5, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 3000, chance = 30, minDamage = -300, maxDamage = -1450, effect = CONST_ME_ENERGY, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2500, chance = 20, minDamage = -200, maxDamage = -480, range = 7, radius = 5, effect = CONST_ME_GREENSHIMMER, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -250, maxDamage = -500, range = 7, radius = 13, effect = CONST_ME_REDNOTE, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -200, maxDamage = -450, radius = 14, effect = CONST_ME_BLUEBUBBLE, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "melee", interval = 3000, chance = 15, minDamage = -100, maxDamage = -200, range = 7, radius = 3, effect = CONST_ME_BLUESHIMMER, target = false},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDNOTE, target = false, speed = -400, duration = 20000},
	{name = "combat", interval = 2000, chance = 15, minDamage = -70, maxDamage = -320, radius = 3, effect = CONST_ME_BLACKSPARK, target = true, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 65,
	armor = 130,
	{name = "combat", interval = 3000, chance = 35, minDamage = 800, maxDamage = 1100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 9000, chance = 15, minDamage = 3800, maxDamage = 4000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 4000, chance = 80, effect = CONST_ME_REDSHIMMER, speed = 470, duration = 6000},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_ENERGYDAMAGE, percent = 80},
	{type = COMBAT_DEATHDAMAGE, percent = 80},
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Demon", chance = 33, interval = 4000, max = 6},
}

mType:register(monster)