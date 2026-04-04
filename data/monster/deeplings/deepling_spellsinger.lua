local mType = Game.createMonsterType("Deepling Spellsinger")
local monster = {}

monster.description = "a deepling spellsinger"
monster.experience = 1000
monster.outfit = {
	lookType = 443,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15208
monster.health = 850
monster.maxHealth = 850
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
	illusionable = true,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 60,
	runHealth = 60,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Jey Obu giotja!!", yell = false},
	{text = "Mmmmmoooaaaaaahaaa!!", yell = false},
}

monster.loot = {
	{id = 2146, chance = 2854}, -- small sapphire
	{id = 2148, chance = 70000, maxCount = 60}, -- gold coin
	{id = 2152, chance = 80000}, -- platinum coin
	{id = 2168, chance = 2439}, -- life ring
	{id = 2667, chance = 3448}, -- fish
	{id = 5895, chance = 498}, -- fish fin
	{id = 13870, chance = 2500}, -- eye of a deepling
	{id = 15400, chance = 2000}, -- deepling staff
	{id = 15403, chance = 813}, -- necklace of the deep
	{id = 15421, chance = 14285}, -- spellsinger's seal
	{id = 15422, chance = 10000}, -- key to the Drowned Library
	{id = 15488, chance = 14285}, -- deepling filet
	{id = 15644, chance = 220}, -- ornate crossbow
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -152, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -180, maxDamage = -350, length = 10, spread = 3, effect = CONST_ME_ICETORNADO, target = false, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -90, maxDamage = -130, radius = 4, effect = CONST_ME_BUBBLES, target = true, type = COMBAT_DROWNDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -140, effect = CONST_ME_SMALLPLANTS, target = true, range = 7, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 10, minDamage = -60, maxDamage = -140, effect = CONST_ME_SMALLPLANTS, target = true, range = 7, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -20},
	{type = COMBAT_ENERGYDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)
