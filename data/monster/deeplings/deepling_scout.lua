local mType = Game.createMonsterType("Deepling Scout")
local monster = {}

monster.description = "a deepling scout"
monster.experience = 160
monster.outfit = {
	lookType = 413,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13839
monster.health = 240
monster.maxHealth = 240
monster.race = "blood"
monster.speed = 200
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 50,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Njaaarh!!", yell = false},
	{text = "Begjone, intrjuder!!", yell = false},
	{text = "Djon't djare stjare injo the eyes of the djeep!", yell = false},
	{text = "Ljeave this sjacred pljace while you cjan", yell = false},
}

monster.loot = {
	{id = 2148, chance = 60000, maxCount = 50}, -- gold coin
	{id = 2149, chance = 121}, -- small emerald
	{id = 2168, chance = 2127}, -- life ring
	{id = 3965, chance = 14285, maxCount = 3}, -- hunting spear
	{id = 5895, chance = 310}, -- fish fin
	{id = 9808, chance = 925}, -- rusty armor
	{id = 9930, chance = 111}, -- flask of rust remover
	{id = 13838, chance = 505}, -- heavy trident
	{id = 13870, chance = 310}, -- eye of a deepling
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -40, maxDamage = -100, shootEffect = CONST_ANI_SPEAR, effect = CONST_ME_BLUE_BUBBLE, target = true, range = 7, type = COMBAT_DROWNDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)
