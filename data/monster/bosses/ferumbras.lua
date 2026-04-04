local mType = Game.createMonsterType("Ferumbras")
local monster = {}

monster.description = "Ferumbras"
monster.experience = 12000
monster.outfit = {
	lookType = 229,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6078
monster.health = 50000
monster.maxHealth = 50000
monster.race = "venom"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 4

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
	targetDistance = 2,
	staticAttackChance = 90,
	runHealth = 2500,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 20,
	{text = "NO ONE WILL STOP ME THIS TIME!", yell = true},
	{text = "THE POWER IS MINE!", yell = true},
	{text = "I returned from death and you dream about defeating me?", yell = false},
	{text = "Witness the first seconds of my eternal world domination!", yell = false},
	{text = "Even in my weakened state I will crush you all!", yell = false},
}

monster.loot = {
	{id = 5903, chance = 100000}, -- ferumbras' hat
	{id = 2148, chance = 98000, maxCount = 184}, -- gold coin
	{id = 9971, chance = 75000, maxCount = 2}, -- gold ingot
	{id = 2522, chance = 26000}, -- great shield
	{id = 8903, chance = 26000}, -- spellbook of lost souls
	{id = 2466, chance = 24000}, -- golden armor
	{id = 2470, chance = 22000}, -- golden legs
	{id = 8902, chance = 22000}, -- spellbook of mind control
	{id = 8868, chance = 22000}, -- velvet mantle
	{id = 2520, chance = 20000}, -- demon shield
	{id = 8885, chance = 20000}, -- divine plate
	{id = 7894, chance = 20000}, -- magma legs
	{id = 2542, chance = 20000}, -- tempest shield
	{id = 2127, chance = 18000}, -- emerald bangle
	{id = 7896, chance = 18000}, -- glacier kilt
	{id = 7895, chance = 18000}, -- lightning legs
	{id = 2539, chance = 18000}, -- phoenix shield
	{id = 8918, chance = 18000}, -- spellbook of dark mysteries
	{id = 7885, chance = 18000}, -- terra legs
	{id = 8930, chance = 16000}, -- emerald sword
	{id = 7405, chance = 16000}, -- havoc blade
	{id = 7451, chance = 16000}, -- shadow sceptre
	{id = 2149, chance = 16000, maxCount = 100}, -- small emerald
	{id = 7632, chance = 14000, maxCount = 5},
	{id = 7633, chance = 14000, maxCount = 5},
	{id = 2472, chance = 14000}, -- magic plate armor
	{id = 2514, chance = 14000}, -- mastermind shield
	{id = 7417, chance = 14000}, -- runed sword
	{id = 8904, chance = 14000}, -- spellscroll of prophecies
	{id = 7427, chance = 12000}, -- chaos mace
	{id = 8926, chance = 12000}, -- demonwing axe
	{id = 8869, chance = 12000}, -- greenwood coat
	{id = 2146, chance = 12000, maxCount = 98}, -- small sapphire
	{id = 2143, chance = 12000, maxCount = 88}, -- white pearl
	{id = 7407, chance = 10000}, -- haunted blade
	{id = 8924, chance = 10000}, -- hellforged axe
	{id = 7411, chance = 10000}, -- ornamented axe
	{id = 2150, chance = 10000, maxCount = 54}, -- small amethyst
	{id = 9970, chance = 10000, maxCount = 87}, -- small topaz
	{id = 7382, chance = 8000}, -- demonrage sword
	{id = 7422, chance = 8000}, -- jade hammer
	{id = 2152, chance = 8000, maxCount = 58}, -- platinum coin
	{id = 7423, chance = 8000}, -- skullcrusher
	{id = 5944, chance = 8000, maxCount = 9}, -- soul orb
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -350, interval = 2000, target = false},
	{name = "combat", type = COMBAT_MANADRAIN, minDamage = -500, maxDamage = -700, interval = 2000, chance = 20, range = 7, target = true, effect = CONST_ME_REDSHIMMER},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -300, maxDamage = -450, interval = 2000, chance = 25, length = 8, spread = 0, target = false, effect = CONST_ME_GREENSPARK},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -450, maxDamage = -500, interval = 2000, chance = 21, radius = 6, target = false, effect = CONST_ME_POFF},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -20, maxDamage = -40, range = 7, target = true},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -900, maxDamage = -1000, interval = 2000, chance = 15, range = 4, radius = 3, target = false},
	{name = "condition", type = CONDITION_ENERGY, interval = 2000, chance = 18, tick = 10000, minDamage = -300, maxDamage = -400, radius = 6, effect = CONST_ME_ENERGY, target = false},
	{name = "condition", type = CONDITION_FIRE, interval = 3000, chance = 20, tick = 10000, minDamage = -500, maxDamage = -600, range = 7, radius = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true},
}

monster.defenses = {
	defense = 120,
	armor = 100,
	{name = "combat", interval = 2000, chance = 10, minDamage = 900, maxDamage = 1500, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 20, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 90},
	{type = COMBAT_HOLYDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Demon", chance = 12, interval = 3000, max = 4},
}

mType:register(monster)