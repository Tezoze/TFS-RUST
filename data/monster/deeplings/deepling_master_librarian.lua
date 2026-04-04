local mType = Game.createMonsterType("Deepling Master Librarian")
local monster = {}

monster.description = "a deepling master librarian"
monster.experience = 1900
monster.outfit = {
	lookType = 443,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15211
monster.health = 1700
monster.maxHealth = 1700
monster.race = "blood"
monster.speed = 190
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
	staticAttackChance = 60,
	runHealth = 250,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Mmmmmoooaaaaaahaaa!!!", yell = false},
}

monster.loot = {
	{id = 2146, chance = 8440, maxCount = 3}, -- small sapphire
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 90000, maxCount = 3}, -- platinum coin
	{id = 2168, chance = 3200}, -- life ring
	{id = 2667, chance = 9090, maxCount = 2}, -- fish
	{id = 5895, chance = 1950}, -- fish fin
	{id = 13870, chance = 9380}, -- eye of a deepling
	{id = 15400, chance = 3130}, -- deepling staff
	{id = 15403, chance = 1330}, -- necklace of the deep
	{id = 15421, chance = 25000}, -- spellsinger's seal
	{id = 15422, chance = 20000}, -- key to the Drowned Library
	{id = 15488, chance = 20000}, -- deepling filet
	{id = 15644, chance = 39}, -- ornate crossbow
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -210, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -260, maxDamage = -450, length = 10, spread = 3, effect = CONST_ME_ICETORNADO, target = false, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -150, maxDamage = -280, radius = 4, effect = CONST_ME_BUBBLES, target = true, type = COMBAT_DROWNDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -80, maxDamage = -140, effect = CONST_ME_SMALLPLANTS, target = true, range = 7, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -60, maxDamage = -140, effect = CONST_ME_SMALLPLANTS, target = true, range = 7, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 20,
	armor = 20,
	{name = "combat", interval = 2000, chance = 15, minDamage = 40, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)
