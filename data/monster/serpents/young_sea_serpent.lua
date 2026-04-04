local mType = Game.createMonsterType("Young Sea Serpent")
local monster = {}

monster.description = "a young sea serpent"
monster.experience = 1000
monster.outfit = {
	lookType = 317,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9879
monster.health = 1050
monster.maxHealth = 1050
monster.race = "blood"
monster.speed = 480
monster.manaCost = 390
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
	staticAttackChance = 85,
	canPushCreatures = true,
	targetDistance = 1,
	runHealth = 400,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "CHHHRRRR", yell = false},
	{text = "HISSSS", yell = false},
}

monster.loot = {
	{id = 2146, chance = 1900, maxCount = 2}, -- small sapphire
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 48000, maxCount = 74}, -- gold coin
	{id = 2165, chance = 1000}, -- stealth ring
	{id = 2177, chance = 300}, -- life crystal
	{id = 2378, chance = 8000}, -- battle axe
	{id = 2394, chance = 40000}, -- morning star
	{id = 2417, chance = 5000}, -- battle hammer
	{id = 7588, chance = 5000}, -- strong health potion
	{id = 7589, chance = 4000}, -- strong mana potion
	{id = 9808, chance = 7940},
	{id = 9809, chance = 7940},
	{id = 10583, chance = 5000}, -- sea serpent scale
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -200, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -10, maxDamage = -250, effect = CONST_ME_SMALLPLANTS, target = false, length = 7, spread = 2, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -80, maxDamage = -250, effect = CONST_ME_ICEATTACK, target = false, length = 7, spread = 2, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 15, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 20,
	{name = "combat", interval = 2000, chance = 30, minDamage = 25, maxDamage = 175, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 400, duration = 5000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -15},
	{type = COMBAT_DEATHDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)