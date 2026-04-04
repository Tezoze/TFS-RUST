local mType = Game.createMonsterType("Ghazbaran")
local monster = {}

monster.description = "Ghazbaran"
monster.experience = 15000
monster.outfit = {
	lookType = 12,
	lookHead = 0,
	lookBody = 123,
	lookLegs = 97,
	lookFeet = 94,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6068
monster.health = 60000
monster.maxHealth = 60000
monster.race = "undead"
monster.speed = 400
monster.manaCost = 0
monster.maxSummons = 4

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
	runHealth = 3500,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 30,
	{text = "COME AND GIVE ME SOME AMUSEMENT", yell = false},
	{text = "IS THAT THE BEST YOU HAVE TO OFFER, TIBIANS?", yell = true},
	{text = "I AM GHAZBARAN OF THE TRIANGLE... AND I AM HERE TO CHALLENGE YOU ALL.", yell = true},
	{text = "FLAWLESS VICTORY!", yell = true},
}

monster.loot = {
	{id = 1984, chance = 20000}, -- blue tome
	{id = 2112, chance = 12500}, -- teddy bear
	{id = 2124, chance = 8333}, -- crystal ring
	{id = 2143, chance = 25000, maxCount = 15}, -- white pearl
	{id = 2144, chance = 11111, maxCount = 14}, -- black pearl
	{id = 2145, chance = 25000, maxCount = 5}, -- small diamond
	{id = 2146, chance = 25000, maxCount = 10}, -- small sapphire
	{id = 2149, chance = 25000, maxCount = 10}, -- small emerald
	{id = 2150, chance = 25000, maxCount = 17}, -- small amethyst
	{id = 2151, chance = 12500, maxCount = 7}, -- talon
	{id = 2152, chance = 100000, maxCount = 69}, -- platinum coin
	{id = 2155, chance = 20000}, -- green gem
	{id = 2158, chance = 14285}, -- blue gem
	{id = 2164, chance = 12500}, -- might ring
	{id = 2165, chance = 12500}, -- stealth ring
	{id = 2174, chance = 11111}, -- strange symbol
	{id = 2177, chance = 12500}, -- life crystal
	{id = 2178, chance = 20000}, -- mind stone
	{id = 2179, chance = 20000}, -- gold ring
	{id = 2214, chance = 20000}, -- ring of healing
	{id = 2447, chance = 11111}, -- twin axe
	{id = 2466, chance = 8333}, -- golden armor
	{id = 2472, chance = 8333}, -- magic plate armor
	{id = 2520, chance = 12500}, -- demon shield
	{id = 2646, chance = 8333}, -- golden boots
	{id = 5954, chance = 33333, maxCount = 2}, -- demon horn
	{id = 6300, chance = 25000}, -- death ring
	{id = 6500, chance = 100000}, -- demonic essence
	{id = 6553, chance = 14285}, -- ruthless axe
	{id = 7368, chance = 12500, maxCount = 44}, -- assassin star
	{id = 7405, chance = 16666}, -- havoc blade
	{id = 7433, chance = 14285}, -- ravenwing
	{id = 7590, chance = 20000}, -- great mana potion
	{id = 7591, chance = 20000}, -- great health potion
	{id = 7896, chance = 8333}, -- glacier kilt
	{id = 8472, chance = 25000}, -- great spirit potion
	{id = 8473, chance = 25000}, -- ultimate health potion
	{id = 8884, chance = 16666}, -- oceanborn leviathan armor
	{id = 8887, chance = 8333}, -- frozen plate
	{id = 8901, chance = 20000}, -- spellbook of warding
	{id = 8902, chance = 11111}, -- spellbook of mind control
	{id = 8903, chance = 16666}, -- spellbook of lost souls
	{id = 8904, chance = 25000}, -- spellscroll of prophecies
	{id = 8918, chance = 20000}, -- spellbook of dark mysteries
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -2191, target = false},
	{name = "melee", interval = 2000, chance = 40, minDamage = -250, maxDamage = -500, range = 7, radius = 6, effect = CONST_ME_BLACKSPARK, target = false},
	{name = "melee", interval = 3000, chance = 34, minDamage = -120, maxDamage = -500, range = 7, radius = 1, shootEffect = CONST_ANI_WHIRLWINDSWORD, effect = CONST_ME_REDSPARK, target = true},
	{name = "combat", interval = 4000, chance = 30, minDamage = -100, maxDamage = -800, effect = CONST_ME_MORTAREA, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 3000, chance = 20, minDamage = -200, maxDamage = -480, range = 14, radius = 5, effect = CONST_ME_POFF, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 4000, chance = 15, minDamage = -100, maxDamage = -650, range = 7, radius = 13, effect = CONST_ME_YELLOWSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 4000, chance = 18, minDamage = -200, maxDamage = -600, radius = 14, effect = CONST_ME_BLUEBUBBLE, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "melee", interval = 3000, chance = 15, minDamage = -200, maxDamage = -750, range = 7, radius = 4, effect = CONST_ME_ENERGYAREA, target = false},
}

monster.defenses = {
	defense = 65,
	armor = 55,
	{name = "combat", interval = 3000, chance = 35, minDamage = 300, maxDamage = 800, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 4000, chance = 80, effect = CONST_ME_REDSHIMMER, speed = 440, duration = 6000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 1},
	{type = COMBAT_DEATHDAMAGE, percent = 1},
	{type = COMBAT_HOLYDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Deathslicer", chance = 20, interval = 4000, max = 4},
}

mType:register(monster)