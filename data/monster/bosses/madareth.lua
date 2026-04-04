local mType = Game.createMonsterType("Madareth")
local monster = {}

monster.description = "Madareth"
monster.experience = 10000
monster.outfit = {
	lookType = 12,
	lookHead = 77,
	lookBody = 116,
	lookLegs = 82,
	lookFeet = 79,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8721
monster.health = 75000
monster.maxHealth = 75000
monster.race = "fire"
monster.speed = 365
monster.manaCost = 0
monster.maxSummons = 0

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
	targetDistance = 1,
	staticAttackChance = 85,
	runHealth = 1200,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I am going to play with yourself!", yell = true},
	{text = "Feel my wrath!", yell = false},
	{text = "No one matches my battle prowess!", yell = false},
	{text = "You will all die!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 150}, -- gold coin
	{id = 9813, chance = 59000},
	{id = 9810, chance = 40000},
	{id = 7443, chance = 33000}, -- bullseye potion
	{id = 7591, chance = 30000}, -- great health potion
	{id = 8472, chance = 30000}, -- great spirit potion
	{id = 7440, chance = 28000}, -- mastermind potion
	{id = 7439, chance = 23000}, -- berserk potion
	{id = 7590, chance = 21000}, -- great mana potion
	{id = 6300, chance = 19000}, -- death ring
	{id = 2183, chance = 19000}, -- hailstorm rod
	{id = 2370, chance = 19000}, -- lute
	{id = 2152, chance = 19000, maxCount = 26}, -- platinum coin
	{id = 2377, chance = 19000}, -- two handed sword
	{id = 7404, chance = 16000}, -- assassin dagger
	{id = 2207, chance = 16000}, -- melee ring
	{id = 8473, chance = 16000}, -- ultimate health potion
	{id = 8910, chance = 16000}, -- underworld rod
	{id = 2207, chance = 14000}, -- melee ring
	{id = 6500, chance = 14000}, -- demonic essence
	{id = 7407, chance = 14000}, -- haunted blade
	{id = 2071, chance = 14000}, -- lyre
	{id = 7418, chance = 14000}, -- nightmare blade
	{id = 8912, chance = 14000}, -- springsprout rod
	{id = 3953, chance = 14000}, -- war drum
	{id = 2187, chance = 11000}, -- wand of inferno
	{id = 8922, chance = 11000}, -- wand of voodoo
	{id = 7416, chance = 9500}, -- bloody edge
	{id = 7449, chance = 9500}, -- crystal sword
	{id = 2214, chance = 9500}, -- ring of healing
	{id = 5954, chance = 7000, maxCount = 2}, -- demon horn
	{id = 2168, chance = 7000}, -- life ring
	{id = 7383, chance = 7000}, -- relic sword
	{id = 2169, chance = 7000}, -- time ring
	{id = 8920, chance = 7000}, -- wand of starstorm
	{id = 2079, chance = 7000}, -- war horn
	{id = 2374, chance = 7000}, -- wooden flute
	{id = 3952, chance = 4700}, -- didgeridoo
	{id = 2213, chance = 4700}, -- dwarven ring
	{id = 2396, chance = 4700}, -- ice rapier
	{id = 7386, chance = 4700}, -- mercenary sword
	{id = 2207, chance = 4700}, -- sword ring
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -2000, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -180, maxDamage = -660, radius = 4, effect = CONST_ME_PURPLEENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -600, maxDamage = -850, effect = CONST_ME_REDSHIMMER, target = false, length = 5, spread = 2, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -200, radius = 4, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 5, minDamage = 0, maxDamage = -250, radius = 5, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 46,
	armor = 48,
	{name = "combat", interval = 3000, chance = 14, minDamage = 400, maxDamage = 900, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = 5},
	{type = COMBAT_DEATHDAMAGE, percent = 95},
	{type = COMBAT_ENERGYDAMAGE, percent = 99},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)