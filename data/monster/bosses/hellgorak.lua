local mType = Game.createMonsterType("Hellgorak")
local monster = {}

monster.description = "Hellgorak"
monster.experience = 10000
monster.outfit = {
	lookType = 12,
	lookHead = 19,
	lookBody = 96,
	lookLegs = 21,
	lookFeet = 81,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6068
monster.health = 25850
monster.maxHealth = 25850
monster.race = "blood"
monster.speed = 330
monster.manaCost = 0
monster.maxSummons = 7

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
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 0,
	{text = "I'll sacrifice yours souls to seven!", yell = false},
	{text = "I'm bad news for you mortals!", yell = false},
	{text = "No man can defeat me!", yell = false},
	{text = "Your puny skills are no match for me.", yell = false},
	{text = "I smell your fear.", yell = false},
	{text = "Delicious!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 200}, -- gold coin
	{id = 9813, chance = 49920},
	{id = 8473, chance = 41750, maxCount = 2}, -- ultimate health potion
	{id = 8901, chance = 31010}, -- spellbook of warding
	{id = 9810, chance = 30560},
	{id = 3962, chance = 29950}, -- beastslayer axe
	{id = 2152, chance = 21790, maxCount = 30}, -- platinum coin
	{id = 8472, chance = 21180}, -- great spirit potion
	{id = 7591, chance = 20570}, -- great health potion
	{id = 2487, chance = 19670}, -- crown armor
	{id = 7590, chance = 16190}, -- great mana potion
	{id = 2144, chance = 14070, maxCount = 25}, -- black pearl
	{id = 2143, chance = 13920, maxCount = 25}, -- white pearl
	{id = 7456, chance = 12860}, -- noble axe
	{id = 2145, chance = 12860, maxCount = 25}, -- small diamond
	{id = 2147, chance = 13010, maxCount = 5}, -- small ruby
	{id = 2125, chance = 12710}, -- crystal necklace
	{id = 2150, chance = 12410, maxCount = 25}, -- small amethyst
	{id = 2133, chance = 11800}, -- ruby necklace
	{id = 2146, chance = 11650, maxCount = 25}, -- small sapphire
	{id = 7894, chance = 11350}, -- magma legs
	{id = 9970, chance = 11200, maxCount = 25}, -- small topaz
	{id = 2149, chance = 10740, maxCount = 25}, -- small emerald
	{id = 2645, chance = 10740}, -- steel boots
	{id = 8871, chance = 10590}, -- focus cape
	{id = 2488, chance = 10140}, -- crown legs
	{id = 8870, chance = 10140}, -- spirit cloak
	{id = 2130, chance = 9680}, -- golden amulet
	{id = 2477, chance = 9530}, -- knight legs
	{id = 5954, chance = 9230, maxCount = 2}, -- demon horn
	{id = 8902, chance = 8770}, -- spellbook of mind control
	{id = 8903, chance = 8620}, -- spellbook of lost souls
	{id = 2656, chance = 8170}, -- blue robe
	{id = 2466, chance = 2870}, -- golden armor
	{id = 7412, chance = 2720}, -- butcher's axe
	{id = 7388, chance = 1970}, -- vile axe
	{id = 8904, chance = 1360}, -- spellscroll of prophecies
	{id = 7453, chance = 610}, -- executioner
	{id = 8926, chance = 450}, -- demonwing axe
	{id = 2470, chance = 450}, -- golden legs
	{id = 8879, chance = 450}, -- voltage armor
	{id = 8918, chance = 300}, -- spellbook of dark mysteries
	{id = 2136, chance = 150}, -- demonbone amulet
	{id = 2415, chance = 100}, -- great axe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -910, target = false},
	{name = "combat", interval = 1000, chance = 11, minDamage = -250, maxDamage = -819, effect = CONST_ME_PURPLEENERGY, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 14, minDamage = -90, maxDamage = -500, radius = 5, effect = CONST_ME_STUN, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 1000, chance = 11, minDamage = -50, maxDamage = -520, radius = 5, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = 0, maxDamage = -150, radius = 7, effect = CONST_ME_POFF, target = false, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 65,
	armor = 70,
	{name = "combat", interval = 1000, chance = 11, minDamage = 400, maxDamage = 900, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = 50},
	{type = COMBAT_EARTHDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = 50},
	{type = COMBAT_DROWNDAMAGE, percent = -50},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Dreadbeast", chance = 10, interval = 2000, max = 7},
}

mType:register(monster)